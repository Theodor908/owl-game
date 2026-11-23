// A sheet of translucent ice over a rectangle of the screen: what is under
// it shows through, blurred and bent a little, with frost crystals and a
// whiter rim. `progress` runs from 0 (no ice) to 1 (covered); the ice grows
// in from the edges along a ragged front, and melts back the same way.
shader_type canvas_item;

uniform sampler2D screen : hint_screen_texture, filter_linear_mipmap;
uniform float progress : hint_range(0.0, 1.0) = 1.0;
// Size of the rectangle in pixels, so the frost pattern is not stretched.
uniform vec2 rect_size = vec2(1000.0, 220.0);
uniform float corner_radius = 40.0;
uniform vec4 tint : source_color = vec4(0.62, 0.86, 1.0, 1.0);
// How much of the tint lies over the cards, at full cover.
uniform float opacity : hint_range(0.0, 1.0) = 0.34;
// How strongly what is under the ice is blurred (a mipmap level) and bent.
uniform float blur : hint_range(0.0, 5.0) = 1.6;
uniform float refraction : hint_range(0.0, 0.02) = 0.004;
// Size of the frost pattern, in pixels per cell.
uniform float grain = 90.0;
uniform float rim_width = 22.0;
uniform float sparkle : hint_range(0.0, 1.0) = 0.5;

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
		mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
	float total = 0.0;
	float amplitude = 0.5;
	for (int octave = 0; octave < 5; octave++) {
		total += noise(p) * amplitude;
		p *= 2.03;
		amplitude *= 0.5;
	}
	return total;
}

// Sharp crystal lines: ridges of the noise.
float crystals(vec2 p) {
	float ridge = 1.0 - abs(noise(p) * 2.0 - 1.0);
	float finer = 1.0 - abs(noise(p * 2.7 + 13.0) * 2.0 - 1.0);
	return pow(ridge, 8.0) * 0.7 + pow(finer, 12.0) * 0.5;
}

// Signed distance to the rounded rectangle, in pixels; negative inside.
float rounded_box(vec2 point, vec2 half_size, float radius) {
	vec2 q = abs(point) - half_size + radius;
	return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

void fragment() {
	vec2 pixel = UV * rect_size;
	float inside = -rounded_box(pixel - rect_size * 0.5, rect_size * 0.5, corner_radius);
	if (inside <= 0.0) {
		discard;
	}
	vec2 cell = pixel / grain;
	// How deep into the rectangle this point is, from 0 at the edge to 1 at
	// the middle, made ragged by noise so the front is not a straight line.
	float depth = clamp(inside / (min(rect_size.x, rect_size.y) * 0.5), 0.0, 1.0);
	float ragged = depth * 0.75 + fbm(cell * 1.3) * 0.45;
	float reach = progress * 1.25;
	float cover = smoothstep(reach, reach - 0.08, ragged);
	if (cover <= 0.001) {
		discard;
	}
	// Near the advancing front the ice is thin and white.
	float front = smoothstep(0.12, 0.0, abs(ragged - reach)) * (1.0 - step(0.999, progress));

	vec2 bend = (vec2(fbm(cell * 2.0), fbm(cell * 2.0 + 7.3)) - 0.5) * refraction;
	vec3 under = textureLod(screen, SCREEN_UV + bend, blur).rgb;
	float frost = fbm(cell * 3.0);
	float lines = crystals(cell * 4.5);
	float rim = smoothstep(rim_width, 0.0, inside);

	// Frost gathers thick and white at the edges; the middle stays clear
	// enough to read the cards through.
	float edge_frost = smoothstep(0.6, 0.0, depth + (frost - 0.5) * 0.35);
	vec3 ice = mix(under, tint.rgb, opacity * (0.55 + 0.45 * edge_frost) + frost * 0.1);
	ice = mix(ice, vec3(0.93, 0.97, 1.0), edge_frost * (0.25 + frost * 0.45));
	ice += vec3(lines) * (0.12 + 0.3 * edge_frost) + vec3(rim) * 0.35 + vec3(front) * 0.5;
	// One soft diagonal sheen, as on a pane of glass.
	float sheen = exp(-pow((UV.x - UV.y * 0.3 - 0.3) * 6.0, 2.0)) * 0.1;
	ice += vec3(sheen);
	// A few points glint, slowly.
	vec2 spark_cell = cell * 6.0;
	float spark_seed = hash(floor(spark_cell));
	float point = smoothstep(0.14, 0.0, length(fract(spark_cell) - 0.5));
	float glint = step(0.975, spark_seed) * point * (0.5 + 0.5 * sin(TIME * 3.0 + spark_seed * 40.0));
	ice += vec3(glint * sparkle);
	COLOR = vec4(ice, cover);
}
