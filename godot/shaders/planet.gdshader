shader_type spatial;

uniform sampler2D terrain_tex : source_color;
uniform sampler2D hydro_a     : source_color;
uniform sampler2D hydro_b     : source_color;
uniform sampler2D ice_a       : source_color;
uniform sampler2D ice_b       : source_color;
uniform float blend : hint_range(0.0, 1.0) = 0.0;
uniform bool show_hydro = true;
uniform bool show_ice   = true;

void fragment() {
    vec2 uv = UV;
    vec4 color = texture(terrain_tex, uv);

    if (show_hydro) {
        vec4 hydro = mix(texture(hydro_a, uv), texture(hydro_b, uv), blend);
        color = mix(color, hydro, hydro.a);
    }

    if (show_ice) {
        vec4 ice = mix(texture(ice_a, uv), texture(ice_b, uv), blend);
        color = mix(color, ice, ice.a);
    }

    ALBEDO = color.rgb;
}
