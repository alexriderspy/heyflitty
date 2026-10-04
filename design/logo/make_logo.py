#!/usr/bin/env python3
"""Generates the Flitty mark: a dotted flight path whose dots grow toward the cursor.

Writes flitty-mark.svg (black), flitty-mark-white.svg and flitty-app-icon.svg
(black mark on a white rounded tile). Run from this directory.
"""
import math


def bezier(t, p0, p1, p2, p3):
    u = 1 - t
    return tuple(u**3 * a + 3 * u * u * t * b + 3 * u * t * t * c + t**3 * d for a, b, c, d in zip(p0, p1, p2, p3))


def evenly_spaced(points_on_curve, count, end_fraction):
    """Picks `count` points spaced evenly by arc length up to `end_fraction` of the curve."""
    samples = [bezier(i / 400, *points_on_curve) for i in range(401)]
    lengths = [0.0]
    for a, b in zip(samples, samples[1:]):
        lengths.append(lengths[-1] + math.dist(a, b))
    total = lengths[-1] * end_fraction
    picked = []
    for i in range(count):
        target = total * i / (count - 1)
        index = next(j for j, length in enumerate(lengths) if length >= target)
        picked.append(samples[index])
    return picked


CURVE = ((9, 55), (11, 32), (22, 18), (36, 17))
POINTER = [(40, 9), (58, 26.5), (48.6, 27.6), (44.2, 37)]


def shapes(color):
    dots = evenly_spaced(CURVE, 5, 0.9)
    circles = [(x, y, 2.4 + 1.6 * i / 4) for i, (x, y) in enumerate(dots)]
    # Center the drawing in the 64x64 box.
    xs = [x - r for x, _, r in circles] + [x + r for x, _, r in circles] + [x for x, _ in POINTER]
    ys = [y - r for _, y, r in circles] + [y + r for _, y, r in circles] + [y for _, y in POINTER]
    dx, dy = 32 - (min(xs) + max(xs)) / 2, 32 - (min(ys) + max(ys)) / 2
    parts = [f'<circle cx="{x + dx:.2f}" cy="{y + dy:.2f}" r="{r:.2f}" fill="{color}"/>' for x, y, r in circles]
    pointer = " L".join(f"{x + dx:.2f} {y + dy:.2f}" for x, y in POINTER)
    parts.append(f'<path d="M{pointer} Z" fill="{color}" stroke="{color}" stroke-width="2.5" stroke-linejoin="round"/>')
    return "".join(parts)


def svg(inner, background=None):
    tile = f'<rect width="64" height="64" rx="14" fill="{background}"/>' if background else ""
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">{tile}{inner}</svg>\n'


if __name__ == "__main__":
    open("flitty-mark.svg", "w").write(svg(shapes("#000000")))
    open("flitty-mark-white.svg", "w").write(svg(shapes("#ffffff")))
    open("flitty-app-icon.svg", "w").write(svg(f'<g transform="translate(9.6 9.6) scale(0.7)">{shapes("#000000")}</g>', background="#ffffff"))
