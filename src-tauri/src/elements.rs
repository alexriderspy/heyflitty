//! The clickable controls in the focused window, from Windows UI Automation.
//! The model picks one by id, so the cursor lands on the control's real bounds
//! instead of a coordinate guessed from pixels.

/// A visible control, in global physical pixels.
#[derive(Clone, Debug)]
pub struct UiElement {
    pub id: usize,
    pub role: &'static str,
    pub name: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl UiElement {
    pub fn center(&self) -> (f64, f64) {
        ((self.left + self.right) as f64 / 2.0, (self.top + self.bottom) as f64 / 2.0)
    }
}

/// Large windows (browsers) expose thousands of nodes; the model only needs the clickable ones.
#[cfg(target_os = "windows")]
const MAX_ELEMENTS: usize = 250;

#[cfg(target_os = "windows")]
pub fn focused_window_elements() -> Result<Vec<UiElement>, String> {
    use uiautomation::types::{ControlType, Handle, TreeScope, UIProperty};
    use uiautomation::UIAutomation;
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let automation = UIAutomation::new().map_err(|error| error.to_string())?;
    // SAFETY: plain Win32 call with no arguments.
    let foreground = unsafe { GetForegroundWindow() };
    let window = automation.element_from_handle(Handle::from(foreground)).map_err(|error| error.to_string())?;

    // Fetch name, type, bounds and visibility in one cross-process round trip.
    let cache = automation.create_cache_request().map_err(|error| error.to_string())?;
    for property in [UIProperty::Name, UIProperty::ControlType, UIProperty::BoundingRectangle, UIProperty::IsOffscreen] {
        cache.add_property(property).map_err(|error| error.to_string())?;
    }
    let everything = automation.create_true_condition().map_err(|error| error.to_string())?;
    let nodes = window.find_all_build_cache(TreeScope::Descendants, &everything, &cache).map_err(|error| error.to_string())?;

    let mut elements = Vec::new();
    for node in nodes {
        let Ok(control_type) = node.get_cached_control_type() else { continue };
        let role = match control_type {
            ControlType::Button => "button",
            ControlType::SplitButton => "button",
            ControlType::MenuItem => "menu item",
            ControlType::ListItem => "list item",
            ControlType::TabItem => "tab",
            ControlType::TreeItem => "tree item",
            ControlType::Hyperlink => "link",
            ControlType::CheckBox => "checkbox",
            ControlType::RadioButton => "radio button",
            ControlType::ComboBox => "dropdown",
            ControlType::Edit => "text box",
            ControlType::Slider => "slider",
            _ => continue,
        };
        let offscreen = node
            .get_cached_property_value(UIProperty::IsOffscreen)
            .ok()
            .and_then(|value| TryInto::<bool>::try_into(value).ok())
            .unwrap_or(true);
        let name = node.get_cached_name().unwrap_or_default().trim().to_string();
        let Ok(bounds) = node.get_cached_bounding_rectangle() else { continue };
        let (left, top, right, bottom) = (bounds.get_left(), bounds.get_top(), bounds.get_right(), bounds.get_bottom());
        if offscreen || name.is_empty() || right <= left || bottom <= top {
            continue;
        }
        elements.push(UiElement { id: elements.len() + 1, role, name: name.chars().take(80).collect(), left, top, right, bottom });
        if elements.len() == MAX_ELEMENTS {
            break;
        }
    }
    Ok(elements)
}

/// UI Automation is Windows-only; elsewhere the model points by coordinates.
#[cfg(not(target_os = "windows"))]
pub fn focused_window_elements() -> Result<Vec<UiElement>, String> {
    Ok(Vec::new())
}
