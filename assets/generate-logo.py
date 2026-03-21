#!/usr/bin/env python3
"""Generate REASONANCE logo: 4 symmetric Lorentzian resonance curves."""

import math

# SVG dimensions
W, H = 200, 200
Y_BOTTOM = 180  # baseline
Y_TOP = 15      # tallest peak reaches here
X_CENTER = W / 2

# Lorentzian: L(x) = gamma^2 / ((x - x0)^2 + gamma^2)
# At x=x0, L=1 (normalized peak). We scale vertically.

# gamma values: sharpest to broadest
gammas = [8, 16, 28, 45]

# x range: full width of viewbox
X_MIN, X_MAX = 0, 200
NUM_POINTS = 201  # odd so we hit center exactly

def lorentzian(x, x0, gamma):
    return (gamma ** 2) / ((x - x0) ** 2 + gamma ** 2)

def make_path(gamma, y_peak):
    """Generate SVG path data for a Lorentzian curve."""
    height = Y_BOTTOM - y_peak  # vertical range for this curve
    points = []
    for i in range(NUM_POINTS):
        x = X_MIN + (X_MAX - X_MIN) * i / (NUM_POINTS - 1)
        y_norm = lorentzian(x, X_CENTER, gamma)
        y = Y_BOTTOM - y_norm * height
        points.append((round(x, 2), round(y, 2)))

    d = f"M {points[0][0]},{points[0][1]}"
    for px, py in points[1:]:
        d += f" L {px},{py}"
    return d

# Scale peaks: sharpest reaches Y_TOP, others proportionally shorter
# All Lorentzians peak at 1.0 when normalized, so we control the visual
# height to create the classic resonance diagram look
peak_heights = [Y_TOP, 40, 75, 115]  # y-coordinates of peaks (lower = taller)

paths = []
for gamma, y_peak in zip(gammas, peak_heights):
    d = make_path(gamma, y_peak)
    paths.append(d)

# Build SVG
svg = f'''<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">
'''

for d in paths:
    svg += f'  <path d="{d}" stroke="#000000" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>\n'

svg += '</svg>\n'

with open('/home/uh1/VIBEPROJECTS/FORGE/forge-ide/assets/reasonance-logo.svg', 'w') as f:
    f.write(svg)

print("Generated reasonance-logo.svg")
print(f"  4 Lorentzian curves with γ = {gammas}")
print(f"  Peak y-positions: {peak_heights}")
