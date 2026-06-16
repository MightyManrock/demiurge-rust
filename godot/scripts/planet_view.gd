extends Node3D

const TEX_WIDTH    := 512 * 3
const TEX_HEIGHT   := 256 * 3
const ORBITAL_PERIOD := 536  # Oros days per year

var renderer: PlanetMapRenderer
var tick: int = 0
var update_interval: int

var terrain_tex: ImageTexture
var hydro_texs: Array[ImageTexture] = []
var ice_texs:   Array[ImageTexture] = []

var shader_mat: ShaderMaterial
var annual_mat: StandardMaterial3D

var _in_planet_view: bool = false

func _ready() -> void:
	renderer = PlanetMapRenderer.new()
	add_child(renderer)
	renderer.initialize()
	update_interval = renderer.update_interval_ticks()

	_build_textures()
	_build_materials()
	_apply_system_view()

func _process(_delta: float) -> void:
	if not _in_planet_view:
		return
	tick += 1
	if tick % update_interval == 0:
		var season_phase  := float(tick % ORBITAL_PERIOD) / float(ORBITAL_PERIOD)
		var snapshot_f    := season_phase * 4.0
		var snap_a        := int(snapshot_f) % 4
		var snap_b        := (snap_a + 1) % 4
		var blend_val     := fmod(snapshot_f, 1.0)
		_update_shader(snap_a, snap_b, blend_val)

# --- LOD API ----------------------------------------------------------------

func enter_planet_view() -> void:
	_in_planet_view = true
	var mesh: MeshInstance3D = $MeshInstance3D
	mesh.set_surface_override_material(0, shader_mat)

func enter_system_view() -> void:
	_in_planet_view = false
	_apply_system_view()

func _on_check_button_toggled(button_pressed: bool) -> void:
	if button_pressed:
		enter_planet_view()
	else:
		enter_system_view()

# --- Internal ----------------------------------------------------------------

func _build_textures() -> void:
	terrain_tex = _make_tex(renderer.terrain_texture(), true)
	for i in range(4):
		hydro_texs.append(_make_tex(renderer.hydrology_texture(i), true))
		ice_texs.append(_make_tex(renderer.ice_texture(i), true))

func _build_materials() -> void:
	var shader := load("res://shaders/planet.gdshader") as Shader
	shader_mat = ShaderMaterial.new()
	shader_mat.shader = shader
	shader_mat.set_shader_parameter("terrain_tex", terrain_tex)
	shader_mat.set_shader_parameter("hydro_a",     hydro_texs[0])
	shader_mat.set_shader_parameter("hydro_b",     hydro_texs[1])
	shader_mat.set_shader_parameter("ice_a",       ice_texs[0])
	shader_mat.set_shader_parameter("ice_b",       ice_texs[1])
	shader_mat.set_shader_parameter("blend",       0.0)

	annual_mat = StandardMaterial3D.new()
	annual_mat.albedo_texture = _make_tex(renderer.annual_texture(), false)

func _apply_system_view() -> void:
	var mesh: MeshInstance3D = $MeshInstance3D
	mesh.set_surface_override_material(0, annual_mat)

func _update_shader(snap_a: int, snap_b: int, blend_val: float) -> void:
	shader_mat.set_shader_parameter("hydro_a", hydro_texs[snap_a])
	shader_mat.set_shader_parameter("hydro_b", hydro_texs[snap_b])
	shader_mat.set_shader_parameter("ice_a",   ice_texs[snap_a])
	shader_mat.set_shader_parameter("ice_b",   ice_texs[snap_b])
	shader_mat.set_shader_parameter("blend",   blend_val)

func _make_tex(bytes: PackedByteArray, rgba: bool) -> ImageTexture:
	var fmt := Image.FORMAT_RGBA8 if rgba else Image.FORMAT_RGB8
	var img := Image.create_from_data(TEX_WIDTH, TEX_HEIGHT, false, fmt, bytes)
	return ImageTexture.create_from_image(img)
