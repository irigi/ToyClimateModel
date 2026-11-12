// Field on a sphere: samples an equirectangular FORMAT_RF texture (row 0 = north edge,
// column 0 = 0°E) and maps it through a 1D colormap. Godot's SphereMesh has UV.x growing
// eastward from +Z and UV.y = 0 at the north pole (+Y), so the texture maps directly.
shader_type spatial;
render_mode unshaded;

uniform sampler2D field : filter_linear, repeat_enable;
uniform sampler2D colormap : filter_linear, repeat_disable;
uniform float vmin = 0.0;
uniform float vmax = 1.0;
// Graticule every 30°; 0 disables it.
uniform float grid_strength = 0.25;

void fragment() {
	float v = texture(field, UV).r;
	float t = clamp((v - vmin) / max(vmax - vmin, 1e-30), 0.0, 1.0);
	vec3 c = texture(colormap, vec2(t, 0.5)).rgb;
	vec2 deg = vec2(UV.x * 360.0, 90.0 - UV.y * 180.0);
	vec2 dist = abs(mod(deg + 15.0, 30.0) - 15.0) / fwidth(deg);
	float line = 1.0 - clamp(min(dist.x, dist.y), 0.0, 1.0);
	ALBEDO = mix(c, vec3(0.0), line * grid_strength);
}
