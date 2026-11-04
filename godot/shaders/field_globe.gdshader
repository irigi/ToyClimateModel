// Field on a sphere: samples an equirectangular FORMAT_RF texture and maps it through a
// 1D colormap. UNTESTED (written in a session without Godot); see docs/godot-integration.md.
shader_type spatial;
render_mode unshaded;

uniform sampler2D field : filter_linear, repeat_enable;
uniform sampler2D colormap : filter_linear, repeat_disable;
uniform float vmin = 0.0;
uniform float vmax = 1.0;
// Longitude offset (0..1) to align the texture seam with the mesh UV seam.
uniform float lon_offset = 0.0;

void fragment() {
	vec2 uv = vec2(UV.x + lon_offset, UV.y);
	float v = texture(field, uv).r;
	float t = clamp((v - vmin) / max(vmax - vmin, 1e-30), 0.0, 1.0);
	ALBEDO = texture(colormap, vec2(t, 0.5)).rgb;
}
