#!/usr/bin/env python3
"""Generates the RemuxDV launcher icon, TV banner, logo and settings icon.

The mark is two offset rounded squares (the base and enhancement layers the app merges) with a
play triangle. Text is converted to paths from Noto Sans Bold (SIL Open Font License).
Writes the Android vector drawables and renders the mipmap PNGs with rsvg-convert.

Usage: python3 branding/generate.py [path/to/NotoSans-Bold.ttf]
"""

import subprocess
import sys
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

RES = Path(__file__).resolve().parent.parent / "app/src/main/res"
FONT = sys.argv[1] if len(sys.argv) > 1 else "/usr/share/fonts/noto/NotoSans-Bold.ttf"

BACKGROUND = "#000B25"
WHITE = "#FFFFFF"
PURPLE = "AA5CC3"
BLUE = "00A4DC"

# Mark geometry in a 40 x 42 box
MARK_W, MARK_H = 40, 42


def rounded_rect(x, y, w, h, r):
	return (
		f"M{x + r},{y}h{w - 2 * r}a{r},{r} 0,0 1,{r},{r}v{h - 2 * r}a{r},{r} 0,0 1,{-r},{r}"
		f"h{-(w - 2 * r)}a{r},{r} 0,0 1,{-r},{-r}v{-(h - 2 * r)}a{r},{r} 0,0 1,{r},{-r}z"
	)


def gradient(x1, y1, x2, y2, alpha="FF"):
	return {"start": (x1, y1), "end": (x2, y2), "colors": (f"#{alpha}{PURPLE}", f"#{alpha}{BLUE}")}


def mark(ox, oy, s, monochrome=False):
	"""Returns [(pathData, fill)] for the mark at (ox, oy) scaled by s."""

	def p(x, y):
		return ox + x * s, oy + y * s

	back = rounded_rect(*p(10, 0), 30 * s, 30 * s, 6 * s)
	front = rounded_rect(*p(0, 12), 30 * s, 30 * s, 6 * s)
	(ax, ay), (bx, by), (cx, cy) = p(10.5, 19.5), p(10.5, 34.5), p(23.5, 27)
	triangle = f"M{ax},{ay}L{bx},{by}L{cx},{cy}z"

	if monochrome:
		return [(back, "#80FFFFFF"), (front + triangle, WHITE, "evenOdd")]

	return [
		(back, gradient(*p(10, 0), *p(40, 30), alpha="88")),
		(front, gradient(*p(0, 12), *p(30, 42))),
		(triangle, WHITE),
	]


class Text:
	def __init__(self, font_path):
		self.font = TTFont(font_path)
		self.glyphs = self.font.getGlyphSet()
		self.cmap = self.font.getBestCmap()
		self.upm = self.font["head"].unitsPerEm

	def width(self, text, size):
		return sum(self.glyphs[self.cmap[ord(c)]].width for c in text) * size / self.upm

	def path(self, text, x, baseline, size):
		scale = size / self.upm
		pen = SVGPathPen(self.glyphs, ntos=lambda v: f"{v:.2f}".rstrip("0").rstrip("."))
		for c in text:
			glyph = self.glyphs[self.cmap[ord(c)]]
			glyph.draw(TransformPen(pen, (scale, 0, 0, -scale, x, baseline)))
			x += glyph.width * scale
		return pen.getCommands()


def wordmark(text, x, y, height, mark_scale, size):
	"""Mark followed by "Remux" in white and "DV" in the gradient, vertically centered on y."""
	shapes = mark(x, y - MARK_H * mark_scale / 2, mark_scale)
	tx = x + MARK_W * mark_scale + size * 0.35
	# Cap height of Noto Sans is ~0.714 em
	baseline = y + size * 0.714 / 2
	shapes.append((text.path("Remux", tx, baseline, size), WHITE))
	dx = tx + text.width("Remux", size)
	shapes.append((text.path("DV", dx, baseline, size), gradient(dx, baseline - size * 0.7, dx + text.width("DV", size), baseline)))
	return shapes


def wordmark_width(text, mark_scale, size):
	return MARK_W * mark_scale + size * 0.35 + text.width("RemuxDV", size)


def vector_xml(width, height, shapes):
	lines = [
		'<vector xmlns:android="http://schemas.android.com/apk/res/android"',
		'    xmlns:aapt="http://schemas.android.com/aapt"',
		f'    android:width="{width}dp"',
		f'    android:height="{height}dp"',
		f'    android:viewportWidth="{width}"',
		f'    android:viewportHeight="{height}">',
	]
	for path, fill, *rule in shapes:
		fill_type = f' android:fillType="{rule[0]}"' if rule else ""
		if isinstance(fill, str):
			lines.append(f'    <path android:fillColor="{fill}"{fill_type} android:pathData="{path}" />')
		else:
			(x1, y1), (x2, y2) = fill["start"], fill["end"]
			c1, c2 = fill["colors"]
			lines += [
				f'    <path{fill_type} android:pathData="{path}">',
				'        <aapt:attr name="android:fillColor">',
				f'            <gradient android:type="linear" android:startX="{x1:g}" android:startY="{y1:g}" android:endX="{x2:g}" android:endY="{y2:g}">',
				f'                <item android:color="{c1}" android:offset="0" />',
				f'                <item android:color="{c2}" android:offset="1" />',
				"            </gradient>",
				"        </aapt:attr>",
				"    </path>",
			]
	lines.append("</vector>")
	return "\n".join(lines) + "\n"


def svg(view_box, width, height, shapes, background=None):
	defs, body = [], []
	if background:
		body.append(f'<rect x="-1000" y="-1000" width="3000" height="3000" fill="{background}"/>')
	for i, (path, fill, *rule) in enumerate(shapes):
		rule_attr = ' fill-rule="evenodd"' if rule else ""
		if isinstance(fill, str):
			color, opacity = (f"#{fill[3:]}", int(fill[1:3], 16) / 255) if len(fill) == 9 else (fill, 1)
			body.append(f'<path d="{path}" fill="{color}" fill-opacity="{opacity:.3f}"{rule_attr}/>')
		else:
			(x1, y1), (x2, y2) = fill["start"], fill["end"]
			stops = "".join(
				f'<stop offset="{o}" stop-color="#{c[3:]}" stop-opacity="{int(c[1:3], 16) / 255:.3f}"/>'
				for o, c in zip((0, 1), fill["colors"])
			)
			defs.append(f'<linearGradient id="g{i}" gradientUnits="userSpaceOnUse" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">{stops}</linearGradient>')
			body.append(f'<path d="{path}" fill="url(#g{i})"{rule_attr}/>')
	return (
		f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" width="{width}" height="{height}">'
		f'<defs>{"".join(defs)}</defs>{"".join(body)}</svg>'
	)


def render_png(svg_data, out):
	subprocess.run(["rsvg-convert", "-o", str(out)], input=svg_data.encode(), check=True)


def main():
	text = Text(FONT)

	# Adaptive icon foreground: 108 x 108 with the mark centered in the 66 dp safe zone
	icon = mark(54 - MARK_W / 2, 54 - MARK_H / 2, 1)
	(RES / "drawable/app_icon_foreground.xml").write_text(vector_xml(108, 108, icon))
	monochrome = mark(54 - MARK_W / 2, 54 - MARK_H / 2, 1, monochrome=True)
	(RES / "drawable/app_icon_foreground_monochrome.xml").write_text(vector_xml(108, 108, monochrome))

	# TV banner foreground: 320 x 180
	banner_mark_scale, banner_size = 1.5, 40
	bx = (320 - wordmark_width(text, banner_mark_scale, banner_size)) / 2
	banner = wordmark(text, bx, 90, 180, banner_mark_scale, banner_size)
	(RES / "drawable/app_banner_foreground.xml").write_text(vector_xml(320, 180, banner))

	# Logo used in the toolbar, splash screen and screensaver: 252 x 72
	logo_mark_scale, logo_size = 1.3, 38
	logo = wordmark(text, (252 - wordmark_width(text, logo_mark_scale, logo_size)) / 2, 36, 72, logo_mark_scale, logo_size)
	# Upstream keeps a drawable-v24 copy, which wins on every device the app supports
	for folder in ("drawable", "drawable-v24"):
		(RES / f"{folder}/app_logo.xml").write_text(vector_xml(252, 72, logo))

	# Settings icon (tinted by the UI): 24 x 24 with the mark scaled to fit
	settings_icon = mark(12 - MARK_W / 4, 12 - MARK_H / 4, 0.5, monochrome=True)
	(RES / "drawable/ic_remuxdv.xml").write_text(vector_xml(24, 24, settings_icon))

	# Legacy (API < 26) icon and the TV banner PNGs; the icon shows the adaptive icon's visible 72 dp
	icon_sizes = {"mdpi": 80, "hdpi": 120, "xhdpi": 160, "xxhdpi": 240, "xxxhdpi": 320}
	for density, size in icon_sizes.items():
		render_png(svg("18 18 72 72", size, size, icon, BACKGROUND), RES / f"mipmap-{density}/app_icon.png")
		render_png(svg("0 0 320 180", size * 2, size * 9 // 8, banner, BACKGROUND), RES / f"mipmap-{density}/app_banner.png")


if __name__ == "__main__":
	main()
