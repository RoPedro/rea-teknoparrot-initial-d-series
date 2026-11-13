#!/usr/bin/env python3
"""Original night mountain-pass illustrations for the launcher's hub.

Dev-time asset authoring only (the outputs, assets/art/*.jpg, are committed;
nothing in the build or the package runs this). Every scene is drawn here
from shapes and a fixed seed: no photos, no game files, no official art, no
logos and no characters. A scene is a winding road at night with guardrails,
headlight beams, tail lights ahead, city lights in the valley and, depending
on the scene, hairpins on the hillside, hydrangeas, fog, a tunnel mouth,
rain or warning chevrons.

usage: python3 -I scripts/art/scenes.py assets/art   (needs rsvg-convert, Pillow)
"""
import math
import os
import random
import subprocess
import sys

W, H = 2560, 1440
HORIZON = 700
VX = W // 2 - 120


def hexrgb(h):
    h = h.lstrip('#')
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def mix(a, b, t):
    a, b = hexrgb(a), hexrgb(b)
    return '#%02x%02x%02x' % tuple(round(x + (y - x) * t) for x, y in zip(a, b))


class Scene:
    def __init__(self, slug, seed, sky_top, sky_glow, light, accent, bend, wiggle=0.0,
                 features=(), flowers=('#7b6cff', '#a98bff', '#5a9bff', '#d98cf0')):
        self.slug, self.seed = slug, seed
        self.sky_top, self.sky_glow, self.light, self.accent = sky_top, sky_glow, light, accent
        self.bend, self.wiggle, self.features, self.flowers = bend, wiggle, set(features), flowers


SCENES = [
    # ID1: dusk over the pass, hydrangeas along the rail, chevrons on the bend
    Scene('id1', 3, '#0a0a1c', '#4a2e6e', '#ffd28a', '#fff1d0', -0.8, 0.0,
          ['hydrangea', 'chevrons', 'city', 'moon']),
    # ID2: forest and fog, teal
    Scene('id2', 5, '#041412', '#124a44', '#9ff2d6', '#e3fff4', 0.7, 0.3,
          ['fog', 'trees', 'tunnel']),
    # ID3: twilight orange, a switchback climbing the hill
    Scene('id3', 8, '#120a14', '#7a3a1c', '#ffb25a', '#ffe6c2', -1.0, 0.5,
          ['switchback', 'city', 'chevrons']),
    # ID4: moonlit long descent, blue
    Scene('id4', 13, '#050b1a', '#1c3f78', '#a8c8ff', '#eef4ff', 0.25, 0.0,
          ['city', 'moon', 'trees']),
    # ID5: rain, magenta lights
    Scene('id5', 17, '#0c0618', '#5a1e6a', '#ff8ae0', '#ffe0f6', 0.55, 0.2,
          ['rain', 'chevrons', 'city']),
    # ID6: deep teal, double hairpin and a moon
    Scene('id6', 19, '#04101a', '#0f4a5e', '#7fe6ff', '#dff8ff', -0.6, 0.6,
          ['switchback', 'moon', 'trees']),
    # ID7: amber tunnel mouth ahead
    Scene('id7', 23, '#100a06', '#6a4410', '#ffc04d', '#fff0c7', -0.35, 0.0,
          ['tunnel', 'trees', 'fog']),
    # ID8: red dawn glow, hydrangeas and a hairpin
    Scene('id8', 29, '#140508', '#7a1e2a', '#ff7a6a', '#ffd9d2', 0.85, 0.4,
          ['hydrangea', 'switchback', 'chevrons'], ('#ff8fb0', '#ffb3c8', '#e66a9a', '#c9a0ff')),
    # Zero: city overlook, cyan
    Scene('zero', 31, '#040a18', '#0e3a6a', '#5ce1ff', '#d4f6ff', 0.95, 0.0,
          ['city', 'chevrons', 'moon']),
    # THE ARCADE Season 3: indigo fog and hydrangeas
    Scene('ta3', 37, '#08081e', '#2e2a78', '#b4a8ff', '#ece8ff', -0.5, 0.3,
          ['hydrangea', 'fog', 'trees']),
    # THE ARCADE Season 5: gold, switchback and a moon
    Scene('ta5', 41, '#0e0a04', '#6e5210', '#ffd24d', '#fff3c4', 0.35, 0.5,
          ['switchback', 'moon', 'city']),
]


# ----------------------------------------------------------------- the road

def centre(s, u):
    """The road's centre line at depth u (0 at the horizon, 1 at the bottom edge)."""
    far = (1 - u) ** 2
    return VX + s.bend * 760 * far + s.wiggle * 220 * math.sin(u * 8.5 + s.seed) * (1 - u) ** 1.2


def half_width(u):
    return 34 + 1010 * u


def ypos(u):
    return HORIZON + (H - HORIZON) * u


def pt(s, u, lateral, height=0.0):
    """Screen point at depth u, `lateral` road half-widths from the centre, `height` above the road."""
    return centre(s, u) + lateral * half_width(u), ypos(u) - height * 520 * u


def depths(n, near=1.0, far=0.045, power=1.0):
    """Equal steps in distance (u is 1/z), near to far."""
    z0, z1 = 1 / near, 1 / far
    return [1 / (z0 + (z1 - z0) * (i / (n - 1)) ** power) for i in range(n)]


def defs(s):
    return f'''<defs>
  <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{s.sky_top}"/>
    <stop offset="0.6" stop-color="{mix(s.sky_top, s.sky_glow, 0.55)}"/>
    <stop offset="1" stop-color="{s.sky_glow}"/>
  </linearGradient>
  <linearGradient id="road" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{mix('#0c1016', s.sky_glow, 0.35)}"/>
    <stop offset="1" stop-color="#05070a"/>
  </linearGradient>
  <linearGradient id="beam" x1="0" y1="1" x2="0" y2="0">
    <stop offset="0" stop-color="{mix('#ffffff', s.accent, 0.35)}" stop-opacity="0.55"/>
    <stop offset="1" stop-color="{mix('#ffffff', s.accent, 0.35)}" stop-opacity="0"/>
  </linearGradient>
  <radialGradient id="vig" cx="0.5" cy="0.5" r="0.75">
    <stop offset="0.55" stop-color="#000" stop-opacity="0"/>
    <stop offset="1" stop-color="#000" stop-opacity="0.78"/>
  </radialGradient>
  <filter id="blur6" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="6"/></filter>
  <filter id="blur18" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="18"/></filter>
  <filter id="blur50" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="50"/></filter>
</defs>'''


def sky(r, s):
    out = [f'<rect width="{W}" height="{H}" fill="url(#sky)"/>']
    for _ in range(220):
        out.append(f'<circle cx="{r.randint(0, W)}" cy="{r.randint(20, HORIZON - 260)}" r="{r.uniform(0.7, 2.0):.1f}" '
                   f'fill="#fff" opacity="{r.uniform(0.2, 0.85):.2f}"/>')
    if 'moon' in s.features:
        mx, my = r.choice([560, 1900]), r.randint(150, 260)
        out.append(f'<circle cx="{mx}" cy="{my}" r="150" fill="{s.light}" opacity="0.18" filter="url(#blur50)"/>')
        out.append(f'<circle cx="{mx}" cy="{my}" r="46" fill="{s.accent}" opacity="0.95"/>')
        out.append(f'<circle cx="{mx + 14}" cy="{my - 6}" r="40" fill="{mix(s.sky_top, s.sky_glow, 0.3)}" opacity="0.28"/>')
    out.append(f'<ellipse cx="{VX}" cy="{HORIZON}" rx="1500" ry="280" fill="{s.light}" opacity="0.14" filter="url(#blur50)"/>')
    return '\n'.join(out)


def ridge(r, s, base, amp, shade, step=70, phase=0.0):
    pts = [f'-10,{HORIZON + 40}']
    x = -10
    while x <= W + 80:
        y = base - amp * (0.5 + 0.5 * math.sin(x / 430 + phase)) - r.uniform(0, amp * 0.35)
        pts.append(f'{x},{y:.0f}')
        x += step
    pts.append(f'{W + 10},{HORIZON + 40}')
    return f'<polygon points="{" ".join(pts)}" fill="{shade}"/>'


def mountains(r, s):
    out = []
    for k, (base, amp) in enumerate([(HORIZON - 150, 230), (HORIZON - 70, 170), (HORIZON - 10, 110)]):
        shade = mix('#020306', s.sky_glow, 0.30 - 0.10 * k)
        out.append(ridge(r, s, base, amp, shade, phase=k * 1.7 + s.seed))
    return '\n'.join(out)


def city_lights(r, s):
    """The town in the valley: clusters of small lights just above the horizon."""
    out = []
    for _ in range(26):
        cx = r.choice([r.randint(60, VX - 520), r.randint(VX + 560, W - 60)])
        cy = HORIZON - r.randint(0, 36)
        for _ in range(r.randint(14, 34)):
            x, y = cx + r.gauss(0, 70), cy + r.gauss(0, 10)
            c = r.choice([s.light, s.accent, '#ffffff', '#ff9a5a'])
            out.append(f'<circle cx="{x:.0f}" cy="{y:.0f}" r="{r.uniform(1.1, 2.6):.1f}" fill="{c}" opacity="{r.uniform(0.5, 1):.2f}"/>')
        out.append(f'<ellipse cx="{cx}" cy="{cy}" rx="110" ry="22" fill="{s.light}" opacity="0.12" filter="url(#blur18)"/>')
    return '\n'.join(out)


def switchback(r, s):
    """A lit hairpin road climbing the hillside on the outside of the bend."""
    side = -1 if s.bend > 0 else 1
    out = [f'<polygon points="{VX + side * 380},{HORIZON + 20} {VX + side * 1200},{HORIZON - 520} '
           f'{VX + side * 2000},{HORIZON - 120} {VX + side * 2000},{HORIZON + 20}" fill="{mix("#020306", s.sky_glow, 0.14)}"/>']
    x, y = VX + side * 520, HORIZON - 10
    pts = [(x, y)]
    for i in range(7):
        x += side * (300 if i % 2 == 0 else -260) * (1 - i * 0.07)
        y -= 62
        pts.append((x, y))
    d = 'M' + ' L'.join(f'{px:.0f},{py:.0f}' for px, py in pts)
    out.append(f'<path d="{d}" stroke="{s.light}" stroke-width="12" fill="none" opacity="0.28" filter="url(#blur6)"/>')
    out.append(f'<path d="{d}" stroke="{mix("#10141a", s.light, 0.25)}" stroke-width="7" fill="none" stroke-linejoin="round"/>')
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        n = 9
        for k in range(n):
            t = k / n
            out.append(f'<circle cx="{x0 + (x1 - x0) * t:.0f}" cy="{y0 + (y1 - y0) * t - 7:.0f}" r="2.4" fill="{s.accent}" opacity="0.95"/>')
    # a car's headlights climbing it
    hx, hy = pts[3]
    for dx in (-5, 5):
        out.append(f'<circle cx="{hx + dx:.0f}" cy="{hy - 4:.0f}" r="3.4" fill="#fffbe6"/>')
    out.append(f'<circle cx="{hx:.0f}" cy="{hy - 4:.0f}" r="26" fill="#fffbe6" opacity="0.35" filter="url(#blur6)"/>')
    return '\n'.join(out)


def tunnel_mouth(r, s):
    u = 0.07
    cx = centre(s, u)
    w, h = 190, 150
    y = ypos(u)
    out = [f'<rect x="{cx - w - 90:.0f}" y="{y - h - 60:.0f}" width="{2 * w + 180}" height="{h + 70}" rx="40" fill="{mix("#020306", s.sky_glow, 0.2)}"/>',
           f'<path d="M{cx - w:.0f},{y + 8:.0f} L{cx - w:.0f},{y - h * 0.55:.0f} Q{cx:.0f},{y - h * 1.5:.0f} {cx + w:.0f},{y - h * 0.55:.0f} L{cx + w:.0f},{y + 8:.0f} Z" fill="#030406"/>',
           f'<path d="M{cx - w:.0f},{y - h * 0.55:.0f} Q{cx:.0f},{y - h * 1.5:.0f} {cx + w:.0f},{y - h * 0.55:.0f}" stroke="{s.light}" stroke-width="5" fill="none" opacity="0.9"/>']
    for k in range(9):
        t = (k + 1) / 10
        a = math.pi * (1 - t)
        lx = cx + math.cos(a) * w * 0.84
        ly = y - h * 0.55 - math.sin(a) * h * 0.55
        out.append(f'<circle cx="{lx:.0f}" cy="{ly:.0f}" r="7" fill="{s.accent}"/>')
    out.append(f'<ellipse cx="{cx:.0f}" cy="{y - h * 0.4:.0f}" rx="{w * 0.9:.0f}" ry="{h * 0.5:.0f}" fill="{s.light}" opacity="0.22" filter="url(#blur18)"/>')
    return '\n'.join(out)


def trees(r, s, n=120):
    """Cedar silhouettes on both banks; nearer ones are bigger."""
    out = []
    col = mix('#010203', s.sky_glow, 0.08)
    items = []
    for _ in range(n):
        u = r.uniform(0.05, 1.0) ** 1.4
        side = r.choice((-1, 1))
        lat = side * r.uniform(1.55, 3.4 + 4 * u)
        x, y = pt(s, u, lat)
        items.append((u, x, y))
    for u, x, y in sorted(items):
        h = 40 + 560 * u
        wd = h * 0.26
        for k in range(4):
            t0 = k / 4
            out.append(f'<polygon points="{x - wd * (1 - t0 * 0.7):.0f},{y - h * t0 * 0.8:.0f} {x:.0f},{y - h * (t0 * 0.8 + 0.42):.0f} '
                       f'{x + wd * (1 - t0 * 0.7):.0f},{y - h * t0 * 0.8:.0f}" fill="{col}"/>')
        out.append(f'<rect x="{x - wd * 0.06:.0f}" y="{y - h * 0.1:.0f}" width="{wd * 0.12:.0f}" height="{h * 0.1:.0f}" fill="{col}"/>')
    return '\n'.join(out)


def road(r, s):
    out = [f'<rect x="0" y="{HORIZON}" width="{W}" height="{H - HORIZON}" fill="{mix("#020305", s.sky_glow, 0.10)}"/>']
    us = depths(60, 1.0, 0.045, 1.0)
    # the asphalt, in strips so the curve is followed
    for u0, u1 in zip(us, us[1:]):
        (xl0, y0), (xr0, _) = pt(s, u0, -1), pt(s, u0, 1)
        (xl1, y1), (xr1, _) = pt(s, u1, -1), pt(s, u1, 1)
        shade = mix('#0b0f15', s.sky_glow, 0.30 * (1 - u0))
        out.append(f'<polygon points="{xl0:.0f},{y0:.0f} {xr0:.0f},{y0:.0f} {xr1:.0f},{y1:.0f} {xl1:.0f},{y1:.0f}" fill="{shade}"/>')
    # the verge beyond the rail
    for side in (-1, 1):
        pts = [pt(s, u, side * 1.32) for u in us] + [pt(s, us[-1], side * 6.0)] + [pt(s, u, side * 6.0) for u in reversed(us)]
        out.append(f'<polygon points="{" ".join(f"{x:.0f},{y:.0f}" for x, y in pts)}" fill="{mix("#030508", s.sky_glow, 0.10)}"/>')
    # edge lines and the centre dashes
    for lat in (-0.93, 0.93):
        pts = [pt(s, u, lat) for u in us]
        out.append(f'<polyline points="{" ".join(f"{x:.0f},{y:.0f}" for x, y in pts)}" stroke="#e8ecf0" stroke-width="6" fill="none" opacity="0.55"/>')
    dash = depths(40, 1.0, 0.05, 1.0)
    for i in range(0, len(dash) - 1, 2):
        u0, u1 = dash[i], dash[i + 1]
        (xa, ya), (xb, yb) = pt(s, u0, 0), pt(s, u1, 0)
        w0, w1 = 2 + 16 * u0, 2 + 16 * u1
        out.append(f'<polygon points="{xa - w0:.0f},{ya:.0f} {xa + w0:.0f},{ya:.0f} {xb + w1:.0f},{yb:.0f} {xb - w1:.0f},{yb:.0f}" '
                   f'fill="#f2e6a8" opacity="{0.35 + 0.5 * u0:.2f}"/>')
    return '\n'.join(out)


def guardrails(r, s):
    out = []
    for side in (-1, 1):
        us = depths(70, 1.0, 0.045, 1.0)
        # the rail: a corrugated band, lit by the headlights near the car
        for u0, u1 in zip(us, us[1:]):
            x0, y0 = pt(s, u0, side * 1.18, 0.10)
            x1, y1 = pt(s, u1, side * 1.18, 0.10)
            t0 = 36 * u0
            t1 = 36 * u1
            lit = min(1.0, 0.25 + u0 * 1.1)
            col = mix('#2a313a', s.accent, lit * 0.55)
            out.append(f'<polygon points="{x0:.0f},{y0 - t0:.0f} {x1:.0f},{y1 - t1:.0f} {x1:.0f},{y1 + t1:.0f} {x0:.0f},{y0 + t0:.0f}" fill="{col}"/>')
            out.append(f'<line x1="{x0:.0f}" y1="{y0 - t0 * 0.2:.0f}" x2="{x1:.0f}" y2="{y1 - t1 * 0.2:.0f}" stroke="{s.accent}" stroke-width="{max(1.0, 3 * u0):.1f}" opacity="{lit * 0.7:.2f}"/>')
        # posts and amber reflectors
        for i, u in enumerate(depths(24, 1.0, 0.05, 1.0)):
            x, y = pt(s, u, side * 1.18, 0.0)
            _, ytop = pt(s, u, side * 1.18, 0.16)
            out.append(f'<line x1="{x:.0f}" y1="{y + 4:.0f}" x2="{x:.0f}" y2="{ytop - 6:.0f}" stroke="#0a0d11" stroke-width="{2 + 12 * u:.1f}"/>')
            if i % 2 == 0:
                out.append(f'<circle cx="{x:.0f}" cy="{ytop + 12 * u:.0f}" r="{1.6 + 7 * u:.1f}" fill="#ffb347"/>')
                out.append(f'<circle cx="{x:.0f}" cy="{ytop + 12 * u:.0f}" r="{4 + 22 * u:.1f}" fill="#ffb347" opacity="0.25" filter="url(#blur6)"/>')
    return '\n'.join(out)


def chevrons(r, s):
    """Yellow arrow boards on the outside of the bend."""
    side = -1 if s.bend > 0 else 1
    out = []
    for u in (0.17, 0.26, 0.40, 0.62):
        x, y = pt(s, u, side * 1.7, 0.0)
        bw, bh = 150 * u + 14, 110 * u + 10
        top = y - 380 * u - 30
        out.append(f'<line x1="{x:.0f}" y1="{top + bh:.0f}" x2="{x:.0f}" y2="{y:.0f}" stroke="#0a0d11" stroke-width="{2 + 9 * u:.1f}"/>')
        out.append(f'<rect x="{x - bw / 2:.0f}" y="{top:.0f}" width="{bw:.0f}" height="{bh:.0f}" rx="{4 + 4 * u:.0f}" fill="#f2c230"/>')
        out.append(f'<rect x="{x - bw / 2:.0f}" y="{top:.0f}" width="{bw:.0f}" height="{bh:.0f}" rx="{4 + 4 * u:.0f}" fill="none" stroke="#101216" stroke-width="{1 + 3 * u:.1f}"/>')
        d = -side
        pts = [(x - d * bw * 0.22, top + bh * 0.22), (x + d * bw * 0.18, top + bh * 0.5), (x - d * bw * 0.22, top + bh * 0.78)]
        out.append(f'<polyline points="{" ".join(f"{px:.0f},{py:.0f}" for px, py in pts)}" stroke="#101216" stroke-width="{2 + 12 * u:.1f}" fill="none" stroke-linejoin="miter"/>')
        out.append(f'<rect x="{x - bw:.0f}" y="{top - bh * 0.4:.0f}" width="{2 * bw:.0f}" height="{bh * 1.8:.0f}" fill="{s.accent}" opacity="0.07" filter="url(#blur18)"/>')
    return '\n'.join(out)


def hydrangeas(r, s):
    """Round flower heads in the verge, lit by the headlights."""
    out = []
    items = []
    for _ in range(46):
        u = r.uniform(0.08, 1.0) ** 1.3
        side = r.choice((-1, 1))
        items.append((u, side, r.uniform(1.42, 2.3 + 2 * u)))
    for u, side, lat in sorted(items):
        x, y = pt(s, u, side * lat, 0.0)
        rad = 12 + 150 * u
        lit = 0.45 + 0.55 * u
        out.append(f'<ellipse cx="{x:.0f}" cy="{y:.0f}" rx="{rad * 1.5:.0f}" ry="{rad * 0.5:.0f}" fill="#06140c" opacity="0.9"/>')
        for _ in range(int(7 + 10 * u)):
            ang = r.uniform(0, 2 * math.pi)
            d = r.uniform(0, rad * 0.8)
            px, py = x + math.cos(ang) * d, y - rad * 0.55 + math.sin(ang) * d * 0.55
            col = mix('#101018', r.choice(s.flowers), lit)
            out.append(f'<circle cx="{px:.0f}" cy="{py:.0f}" r="{rad * r.uniform(0.18, 0.34):.0f}" fill="{col}"/>')
        out.append(f'<ellipse cx="{x:.0f}" cy="{y - rad * 0.5:.0f}" rx="{rad:.0f}" ry="{rad * 0.6:.0f}" fill="{s.flowers[0]}" opacity="{0.10 * lit:.2f}" filter="url(#blur18)"/>')
    return '\n'.join(out)


def fog(r, s):
    out = []
    for k in range(5):
        y = HORIZON - 30 + k * 70 + r.randint(-20, 20)
        out.append(f'<ellipse cx="{r.randint(300, W - 300)}" cy="{y}" rx="{r.randint(700, 1300)}" ry="{r.randint(28, 60)}" '
                   f'fill="{mix(s.sky_glow, "#ffffff", 0.35)}" opacity="{r.uniform(0.10, 0.2):.2f}" filter="url(#blur50)"/>')
    return '\n'.join(out)


def headlights(r, s):
    """Two beams from the viewer's car, a pool of light on the asphalt."""
    out = []
    for dx in (-1, 1):
        x0, x1 = centre(s, 1.0) + dx * 430, centre(s, 0.2) + dx * 70
        out.append(f'<polygon points="{x0 - 90:.0f},{H} {x0 + 90:.0f},{H} {x1 + 22:.0f},{ypos(0.2):.0f} {x1 - 22:.0f},{ypos(0.2):.0f}" '
                   f'fill="url(#beam)" opacity="0.62" filter="url(#blur18)"/>')
    out.append(f'<ellipse cx="{centre(s, 0.78):.0f}" cy="{ypos(0.78):.0f}" rx="520" ry="95" fill="{s.accent}" opacity="0.15" filter="url(#blur50)"/>')
    return '\n'.join(out)


def tail_lights(r, s):
    """The car ahead: two red lamps, a faint body, and their glow on the road."""
    out = []
    u = 0.26
    cx, y = pt(s, u, 0.0)
    wd = 150 * u
    out.append(f'<rect x="{cx - wd * 1.1:.0f}" y="{y - wd * 0.85:.0f}" width="{wd * 2.2:.0f}" height="{wd * 0.85:.0f}" rx="{wd * 0.18:.0f}" fill="#06080b" opacity="0.92"/>')
    out.append(f'<ellipse cx="{cx:.0f}" cy="{y + 6:.0f}" rx="{wd * 1.7:.0f}" ry="{wd * 0.3:.0f}" fill="#ff2a1f" opacity="0.28" filter="url(#blur18)"/>')
    for dx in (-1, 1):
        lx = cx + dx * wd * 0.78
        out.append(f'<circle cx="{lx:.0f}" cy="{y - wd * 0.42:.0f}" r="{wd * 0.34:.0f}" fill="#ff2a1f" opacity="0.5" filter="url(#blur6)"/>')
        out.append(f'<rect x="{lx - wd * 0.2:.0f}" y="{y - wd * 0.5:.0f}" width="{wd * 0.4:.0f}" height="{wd * 0.16:.0f}" rx="2" fill="#ff5a4a"/>')
    # a second pair, far ahead on the bend
    u2 = 0.1
    cx2, y2 = pt(s, u2, 0.2)
    for dx in (-1, 1):
        out.append(f'<circle cx="{cx2 + dx * 9:.0f}" cy="{y2 - 6:.0f}" r="4" fill="#ff3b30"/>')
        out.append(f'<circle cx="{cx2 + dx * 9:.0f}" cy="{y2 - 6:.0f}" r="14" fill="#ff3b30" opacity="0.4" filter="url(#blur6)"/>')
    return '\n'.join(out)


def rain(r):
    out = []
    for _ in range(1000):
        x, y = r.randint(0, W), r.randint(0, H)
        out.append(f'<line x1="{x}" y1="{y}" x2="{x - 7}" y2="{y + 38}" stroke="#d0d8ff" stroke-width="1.3" opacity="{r.uniform(0.08, 0.3):.2f}"/>')
    return '\n'.join(out)


def svg(s):
    r = random.Random(s.seed)
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">', defs(s),
             sky(r, s), mountains(r, s)]
    if 'city' in s.features:
        parts.append(city_lights(r, s))
    if 'switchback' in s.features:
        parts.append(switchback(r, s))
    if 'tunnel' in s.features:
        parts.append(tunnel_mouth(r, s))
    parts.append(road(r, s))
    if 'trees' in s.features:
        parts.append(trees(r, s))
    if 'fog' in s.features:
        parts.append(fog(r, s))
    if 'hydrangea' in s.features:
        parts.append(hydrangeas(r, s))
    parts.append(guardrails(r, s))
    if 'chevrons' in s.features:
        parts.append(chevrons(r, s))
    parts.append(headlights(r, s))
    parts.append(tail_lights(r, s))
    if 'rain' in s.features:
        parts.append(rain(r))
    parts.append(f'<rect width="{W}" height="{H}" fill="url(#vig)"/>')
    parts.append('</svg>')
    return '\n'.join(parts)


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    from PIL import Image
    for s in SCENES:
        svg_path = os.path.join(out, s.slug + '.svg')
        png_path = os.path.join(out, s.slug + '.png')
        with open(svg_path, 'w') as f:
            f.write(svg(s))
        subprocess.run(['rsvg-convert', '-o', png_path, svg_path], check=True)
        im = Image.open(png_path).convert('RGB')
        # light film grain, from the fixed generator so the files repeat
        grain = Image.frombytes('L', im.size, random.Random(s.seed).randbytes(im.size[0] * im.size[1]))
        im = Image.blend(im, grain.convert('RGB'), 0.03)
        im.resize((1920, 1080), Image.LANCZOS).save(os.path.join(out, s.slug + '.jpg'), quality=86, optimize=True, progressive=True)
        im.resize((512, 288), Image.LANCZOS).save(os.path.join(out, s.slug + '-tile.jpg'), quality=86)
        os.remove(png_path)
        os.remove(svg_path)
        print(s.slug)


if __name__ == '__main__':
    main()
