//! Push-to-talk microphone capture: records while the hotkey is held and
//! returns 16 kHz mono WAV, the format every transcription API accepts.

use std::io::Cursor;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const TARGET_SAMPLE_RATE: u32 = 16_000;
/// Releases shorter than this are accidental taps, not questions.
const MINIMUM_RECORDING_SECONDS: f32 = 0.3;

pub struct Recording {
    pub wav: Vec<u8>,
    pub seconds: f32,
}

enum Control {
    Stop(mpsc::Sender<Result<Recording, String>>),
}

/// Owns the capture thread; cpal streams are not Send on every platform,
/// so the stream lives and dies on its own thread.
pub struct Recorder {
    control: Mutex<Option<mpsc::Sender<Control>>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self { control: Mutex::new(None) }
    }

    pub fn start(&self) -> Result<(), String> {
        let mut control_slot = self.control.lock().expect("recorder lock");
        if control_slot.is_some() {
            return Ok(());
        }
        let (control_sender, control_receiver) = mpsc::channel::<Control>();
        let (ready_sender, ready_receiver) = mpsc::channel::<Result<(), String>>();
        thread::spawn(move || run_capture(control_receiver, ready_sender));
        ready_receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "microphone did not start".to_string())??;
        *control_slot = Some(control_sender);
        Ok(())
    }

    pub fn stop(&self) -> Result<Recording, String> {
        let control_sender = self.control.lock().expect("recorder lock").take().ok_or("not recording")?;
        let (result_sender, result_receiver) = mpsc::channel();
        control_sender.send(Control::Stop(result_sender)).map_err(|_| "recorder stopped unexpectedly")?;
        result_receiver.recv_timeout(Duration::from_secs(3)).map_err(|_| "recorder did not finish".to_string())?
    }
}

fn run_capture(control_receiver: mpsc::Receiver<Control>, ready_sender: mpsc::Sender<Result<(), String>>) {
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let stream_and_rate = open_input_stream(samples.clone());
    let (stream, device_sample_rate) = match stream_and_rate {
        Ok(opened) => opened,
        Err(error) => {
            let _ = ready_sender.send(Err(error));
            return;
        }
    };
    let _ = ready_sender.send(Ok(()));

    if let Ok(Control::Stop(result_sender)) = control_receiver.recv() {
        drop(stream);
        let mono = std::mem::take(&mut *samples.lock().expect("samples lock"));
        let seconds = mono.len() as f32 / device_sample_rate as f32;
        let result = if seconds < MINIMUM_RECORDING_SECONDS {
            Err("too short".to_string())
        } else {
            encode_wav(&resample_linear(&mono, device_sample_rate, TARGET_SAMPLE_RATE)).map(|wav| Recording { wav, seconds })
        };
        let _ = result_sender.send(result);
    }
}

fn open_input_stream(samples: Arc<Mutex<Vec<f32>>>) -> Result<(cpal::Stream, u32), String> {
    let device = cpal::default_host().default_input_device().ok_or("no microphone found")?;
    let config = device.default_input_config().map_err(|error| error.to_string())?;
    let channels = config.channels() as usize;
    let sample_rate = config.sample_rate();
    let on_error = |error| eprintln!("[flitty] microphone error: {error}");

    // Downmix every frame to mono as it arrives.
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            config.into(),
            move |data: &[f32], _| push_mono(&samples, data.chunks(channels).map(|frame| frame.iter().sum::<f32>() / channels as f32)),
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            config.into(),
            move |data: &[i16], _| {
                push_mono(
                    &samples,
                    data.chunks(channels).map(|frame| frame.iter().map(|&value| value as f32 / i16::MAX as f32).sum::<f32>() / channels as f32),
                )
            },
            on_error,
            None,
        ),
        other => return Err(format!("unsupported microphone sample format {other:?}")),
    }
    .map_err(|error| error.to_string())?;
    stream.play().map_err(|error| error.to_string())?;
    Ok((stream, sample_rate))
}

fn push_mono(samples: &Mutex<Vec<f32>>, frames: impl Iterator<Item = f32>) {
    samples.lock().expect("samples lock").extend(frames);
}

/// Linear interpolation is plenty for speech headed to a transcription model.
fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_length = (input.len() as f64 / ratio) as usize;
    (0..output_length)
        .map(|index| {
            let source_position = index as f64 * ratio;
            let left = source_position.floor() as usize;
            let right = (left + 1).min(input.len() - 1);
            let fraction = (source_position - left as f64) as f32;
            input[left] * (1.0 - fraction) + input[right] * fraction
        })
        .collect()
}

fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec { channels: 1, sample_rate: TARGET_SAMPLE_RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut buffer, spec).map_err(|error| error.to_string())?;
    for &sample in samples {
        writer.write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).map_err(|error| error.to_string())?;
    }
    writer.finalize().map_err(|error| error.to_string())?;
    Ok(buffer.into_inner())
}
