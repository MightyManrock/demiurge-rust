shader_type spatial;
render_mode unshaded;

uniform vec3 core_color    : source_color = vec3(1.0, 1.0, 0.95);
uniform vec3 mid_color     : source_color = vec3(1.0, 0.84, 0.25);
uniform vec3 edge_color    : source_color = vec3(1.0, 0.56, 0.0);
uniform float emission_energy = 3.0;

void fragment() {
    float rim = clamp(dot(NORMAL, VIEW), 0.0, 1.0);
    float t   = 1.0 - rim;
    vec3 col  = mix(core_color, mix(mid_color, edge_color, t), t * t);
    ALBEDO    = col;
    EMISSION  = col * emission_energy;
}
