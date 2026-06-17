extends Node3D

# --- Constants ---------------------------------------------------------------
const TEX_WIDTH          := 512 * 3
const TEX_HEIGHT         := 256 * 3
const ORBITAL_PERIOD     := 536
const ORBIT_RADIUS       := 8.0
const PLANET_PROXY_SCALE := 0.18
const ORBIT_CAM_DIST     := 2.5
const STAR_SCALE_FACTOR  := 1.2
const SYS_ORTHO_SIZE     := 16.0
const SYS_CAM_POS        := Vector3(0.0, 18.0, 12.0)
const SECS_PER_TICK      := 0.1     # real seconds per game day; raise to slow down

# Star color by kind id: BlueGiant=0 WhiteStar=1 YellowDwarf=2
#                        OrangeDwarf=3 RedDwarf=4 RedGiant=5 WhiteDwarf=6
const STAR_COLORS: Array[Color] = [
	Color(0.63, 0.75, 1.0),
	Color(1.0,  1.0,  1.0),
	Color(1.0,  0.82, 0.25),
	Color(1.0,  0.50, 0.19),
	Color(1.0,  0.31, 0.13),
	Color(1.0,  0.19, 0.06),
	Color(0.88, 0.91, 1.0),
]

# --- Renderer / textures / materials -----------------------------------------
var renderer: PlanetMapRenderer
var tick: int = 0
var update_interval: int

var terrain_tex: ImageTexture
var hydro_texs: Array[ImageTexture] = []
var ice_texs:   Array[ImageTexture] = []
var shader_mat: ShaderMaterial
var annual_mat: StandardMaterial3D

# --- Mode state --------------------------------------------------------------
var _in_planet_view: bool = false
var _transitioning:  bool = false
var _ticks_paused:   bool = false
var _oros_selected:  bool = false

# --- Arcball state -----------------------------------------------------------
var _yaw:          float = 0.0
var _pitch:        float = 0.3
var _orbit_radius: float = ORBIT_CAM_DIST
var _drag_active:  bool  = false
var _last_mouse:   Vector2 = Vector2.ZERO

# --- System camera pan state -------------------------------------------------
var _sys_pan_active: bool    = false
var _sys_last_mouse: Vector2 = Vector2.ZERO
var _tick_accum:     float   = 0.0

# =============================================================================

func _ready() -> void:
	renderer = PlanetMapRenderer.new()
	add_child(renderer)
	renderer.initialize()
	update_interval = renderer.update_interval_ticks()

	_build_textures()
	_build_materials()
	_setup_star()
	_setup_orbital_ring()
	_setup_oros_proxy()
	_enter_system_view_immediate()

func _process(delta: float) -> void:
	if _transitioning:
		return
	if not _ticks_paused:
		_tick_accum += delta
		while _tick_accum >= SECS_PER_TICK:
			_tick_accum -= SECS_PER_TICK
			tick += 1
			_update_orbital_position()
			if _in_planet_view and tick % update_interval == 0:
				_update_season_shader()
	if _in_planet_view:
		_process_planet_camera(delta)
	else:
		_process_system_camera(delta)

# --- Input -------------------------------------------------------------------

func _input(event: InputEvent) -> void:
	if _in_planet_view or _transitioning:
		return
	if event is InputEventMouseButton:
		var cam := $Camera3D as Camera3D
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			cam.size = clamp(cam.size - 1.5, 4.0, 40.0)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			cam.size = clamp(cam.size + 1.5, 4.0, 40.0)

func _unhandled_input(event: InputEvent) -> void:
	if _transitioning:
		return
	if _in_planet_view:
		if event is InputEventMouseButton:
			if event.button_index == MOUSE_BUTTON_LEFT:
				_drag_active = event.pressed
				if event.pressed:
					_last_mouse = event.position
			elif event.button_index == MOUSE_BUTTON_WHEEL_UP:
				_orbit_radius = clamp(_orbit_radius - 0.2, 1.3, 6.0)
			elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				_orbit_radius = clamp(_orbit_radius + 0.2, 1.3, 6.0)

# --- Signal handlers ---------------------------------------------------------

func _on_zoom_in_pressed() -> void:
	if _transitioning:
		return
	_transitioning = true
	_ticks_paused  = true
	($UI/ZoomInButton as Button).visible = false

	var cam      := $Camera3D as Camera3D
	var oros_pos := ($SystemView/OrosProxy as MeshInstance3D).global_position
	var target_pos := oros_pos + Vector3(
		sin(_yaw) * cos(_pitch),
		sin(_pitch),
		cos(_yaw) * cos(_pitch)
	) * _orbit_radius

	cam.projection = Camera3D.PROJECTION_PERSPECTIVE
	cam.fov        = 70.0

	var tween := create_tween().set_ease(Tween.EASE_IN_OUT).set_trans(Tween.TRANS_CUBIC)
	tween.tween_property(cam, "global_position", target_pos, 1.2)
	tween.parallel().tween_method(_look_toward.bind(oros_pos), 0.0, 1.0, 1.2)
	get_tree().create_timer(0.6).timeout.connect(_mid_zoom_in.bind(oros_pos))
	tween.finished.connect(_finish_zoom_in)

func _on_back_pressed() -> void:
	if _transitioning:
		return
	_transitioning = true
	_ticks_paused  = true
	($UI/BackButton as Button).visible = false

	var cam      := $Camera3D as Camera3D
	var oros_pos := ($PlanetSphere as MeshInstance3D).global_position
	($PlanetSphere as MeshInstance3D).set_surface_override_material(0, annual_mat)

	var tween := create_tween().set_ease(Tween.EASE_IN_OUT).set_trans(Tween.TRANS_CUBIC)
	tween.tween_property(cam, "global_position", SYS_CAM_POS, 1.2)
	tween.parallel().tween_method(_look_toward.bind(oros_pos), 0.0, 1.0, 1.2)
	get_tree().create_timer(0.6).timeout.connect(_mid_zoom_out)
	tween.finished.connect(_finish_zoom_out)

func _on_oros_proxy_input_event(_camera: Node, event: InputEvent,
		_pos: Vector3, _normal: Vector3, _idx: int) -> void:
	if _in_planet_view or _transitioning:
		return
	if event is InputEventMouseButton \
			and event.button_index == MOUSE_BUTTON_LEFT \
			and event.pressed:
		_oros_selected = true
		($UI/ZoomInButton as Button).visible = true

# --- Transition helpers ------------------------------------------------------

func _look_toward(_t: float, target: Vector3) -> void:
	($Camera3D as Camera3D).look_at(target, Vector3.UP)

func _mid_zoom_in(oros_pos: Vector3) -> void:
	$SystemView.visible   = false
	$PlanetSphere.visible = true
	($PlanetSphere as MeshInstance3D).global_position = oros_pos

func _finish_zoom_in() -> void:
	($PlanetSphere as MeshInstance3D).set_surface_override_material(0, shader_mat)
	_in_planet_view = true
	_transitioning  = false
	_ticks_paused   = false
	($UI/BackButton as Button).visible = true

func _mid_zoom_out() -> void:
	$PlanetSphere.visible = false
	$SystemView.visible   = true

func _finish_zoom_out() -> void:
	var cam    := $Camera3D as Camera3D
	cam.projection = Camera3D.PROJECTION_ORTHOGONAL
	cam.size       = SYS_ORTHO_SIZE
	cam.position   = SYS_CAM_POS
	cam.look_at(Vector3.ZERO, Vector3.UP)
	_in_planet_view = false
	_transitioning  = false
	_ticks_paused   = false
	_oros_selected  = false

# --- Camera ------------------------------------------------------------------

func _enter_system_view_immediate() -> void:
	var cam    := $Camera3D as Camera3D
	cam.projection = Camera3D.PROJECTION_ORTHOGONAL
	cam.size       = SYS_ORTHO_SIZE
	cam.position   = SYS_CAM_POS
	cam.look_at(Vector3.ZERO, Vector3.UP)
	$SystemView.visible   = true
	$PlanetSphere.visible = false
	($UI/ZoomInButton as Button).visible = false
	($UI/BackButton as Button).visible   = false

func _process_system_camera(_delta: float) -> void:
	if Input.is_mouse_button_pressed(MOUSE_BUTTON_RIGHT):
		var mouse_pos := get_viewport().get_mouse_position()
		if not _sys_pan_active:
			_sys_pan_active  = true
			_sys_last_mouse  = mouse_pos
		else:
			var dm       := mouse_pos - _sys_last_mouse
			_sys_last_mouse   = mouse_pos
			var pan_speed := ($Camera3D as Camera3D).size * 0.002
			$Camera3D.position.x -= dm.x * pan_speed
			$Camera3D.position.z -= dm.y * pan_speed * 0.6
	else:
		_sys_pan_active = false

func _process_planet_camera(delta: float) -> void:
	var speed := 1.2 * delta
	if Input.is_key_pressed(KEY_A): _yaw   -= speed
	if Input.is_key_pressed(KEY_D): _yaw   += speed
	if Input.is_key_pressed(KEY_W): _pitch += speed
	if Input.is_key_pressed(KEY_S): _pitch -= speed
	_pitch = clamp(_pitch, -1.38, 1.38)

	if _drag_active:
		var m  := get_viewport().get_mouse_position()
		var dm := m - _last_mouse
		_last_mouse = m
		_yaw   += dm.x * 0.005
		_pitch -= dm.y * 0.005
		_pitch  = clamp(_pitch, -1.38, 1.38)

	var oros_pos := ($SystemView/OrosProxy as MeshInstance3D).global_position
	var offset   := Vector3(
		sin(_yaw) * cos(_pitch),
		sin(_pitch),
		cos(_yaw) * cos(_pitch)
	) * _orbit_radius
	var cam := $Camera3D as Camera3D
	cam.global_position = oros_pos + offset
	cam.look_at(oros_pos, Vector3.UP)

# --- Setup -------------------------------------------------------------------

func _setup_star() -> void:
	var kind_id := renderer.star_kind_id()
	var lum     := renderer.star_luminosity()
	var radius  := renderer.star_radius()
	var col     := STAR_COLORS[clamp(kind_id, 0, STAR_COLORS.size() - 1)]
	var base_scale := radius * STAR_SCALE_FACTOR

	var star_mat := ShaderMaterial.new()
	star_mat.shader = load("res://shaders/star.gdshader") as Shader
	star_mat.set_shader_parameter("core_color",      Color(1.0, 1.0, 0.97))
	star_mat.set_shader_parameter("mid_color",       col)
	star_mat.set_shader_parameter("edge_color",      Color(col.r * 0.6, col.g * 0.6, col.b * 0.6))
	star_mat.set_shader_parameter("emission_energy", lum * 3.0)

	var star := $SystemView/Star as MeshInstance3D
	star.scale = Vector3.ONE * base_scale
	star.set_surface_override_material(0, star_mat)

	var inner_mat                          := StandardMaterial3D.new()
	inner_mat.emission_enabled              = true
	inner_mat.emission                      = col
	inner_mat.emission_energy_multiplier    = lum * 1.5
	inner_mat.albedo_color                  = Color(0.0, 0.0, 0.0, 0.0)
	inner_mat.transparency                  = BaseMaterial3D.TRANSPARENCY_ALPHA
	inner_mat.cull_mode                     = BaseMaterial3D.CULL_FRONT
	inner_mat.shading_mode                  = BaseMaterial3D.SHADING_MODE_UNSHADED
	var inner := $SystemView/StarHaloInner as MeshInstance3D
	inner.scale = Vector3.ONE * base_scale * 1.4
	inner.set_surface_override_material(0, inner_mat)

	var outer_mat                          := inner_mat.duplicate() as StandardMaterial3D
	outer_mat.emission_energy_multiplier    = lum * 0.5
	var outer := $SystemView/StarHaloOuter as MeshInstance3D
	outer.scale = Vector3.ONE * base_scale * 2.2
	outer.set_surface_override_material(0, outer_mat)

	($DirectionalLight3D as DirectionalLight3D).light_color = col

func _setup_orbital_ring() -> void:
	var mesh        := TorusMesh.new()
	mesh.inner_radius = ORBIT_RADIUS - 0.05
	mesh.outer_radius = ORBIT_RADIUS + 0.05

	var mat                       := StandardMaterial3D.new()
	mat.emission_enabled           = true
	mat.emission                   = Color(0.4, 0.6, 1.0)
	mat.emission_energy_multiplier = 0.4
	mat.albedo_color               = Color(0.0, 0.0, 0.0, 0.0)
	mat.transparency               = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.shading_mode               = BaseMaterial3D.SHADING_MODE_UNSHADED

	var ring    := $SystemView/OrbitalRing as MeshInstance3D
	ring.mesh   = mesh
	ring.set_surface_override_material(0, mat)

func _setup_oros_proxy() -> void:
	var proxy       := $SystemView/OrosProxy as MeshInstance3D
	proxy.scale      = Vector3.ONE * PLANET_PROXY_SCALE
	proxy.set_surface_override_material(0, annual_mat)

	# Override collision shape in code so editor-set transforms don't double-scale.
	# Radius is in OrosProxy local space (before the 0.18 node scale) — 1.0 gives
	# a generous click target without looking wrong visually.
	var col_shape  := $SystemView/OrosProxy/Area3D/CollisionShape3D as CollisionShape3D
	var sphere     := SphereShape3D.new()
	sphere.radius   = 1.0
	col_shape.shape     = sphere
	col_shape.transform = Transform3D.IDENTITY

	_update_orbital_position()

func _update_orbital_position() -> void:
	var angle := float(tick % ORBITAL_PERIOD) / float(ORBITAL_PERIOD) * TAU
	($SystemView/OrosProxy as MeshInstance3D).position = Vector3(
		cos(angle) * ORBIT_RADIUS, 0.0, sin(angle) * ORBIT_RADIUS
	)

# --- Textures / materials ----------------------------------------------------

func _build_textures() -> void:
	terrain_tex = _make_tex(renderer.terrain_texture(), true)
	for i in range(4):
		hydro_texs.append(_make_tex(renderer.hydrology_texture(i), true))
		ice_texs.append(_make_tex(renderer.ice_texture(i), true))

func _build_materials() -> void:
	var planet_shader := load("res://shaders/planet.gdshader") as Shader
	shader_mat        = ShaderMaterial.new()
	shader_mat.shader = planet_shader
	shader_mat.set_shader_parameter("terrain_tex", terrain_tex)
	shader_mat.set_shader_parameter("hydro_a",     hydro_texs[0])
	shader_mat.set_shader_parameter("hydro_b",     hydro_texs[1])
	shader_mat.set_shader_parameter("ice_a",       ice_texs[0])
	shader_mat.set_shader_parameter("ice_b",       ice_texs[1])
	shader_mat.set_shader_parameter("blend",       0.0)

	annual_mat                = StandardMaterial3D.new()
	annual_mat.albedo_texture = _make_tex(renderer.annual_texture(), false)

func _update_season_shader() -> void:
	var season_phase := float(tick % ORBITAL_PERIOD) / float(ORBITAL_PERIOD)
	var snapshot_f   := season_phase * 4.0
	var snap_a       := int(snapshot_f) % 4
	var snap_b       := (snap_a + 1) % 4
	var blend_val    := fmod(snapshot_f, 1.0)
	shader_mat.set_shader_parameter("hydro_a", hydro_texs[snap_a])
	shader_mat.set_shader_parameter("hydro_b", hydro_texs[snap_b])
	shader_mat.set_shader_parameter("ice_a",   ice_texs[snap_a])
	shader_mat.set_shader_parameter("ice_b",   ice_texs[snap_b])
	shader_mat.set_shader_parameter("blend",   blend_val)

func _make_tex(bytes: PackedByteArray, rgba: bool) -> ImageTexture:
	var fmt := Image.FORMAT_RGBA8 if rgba else Image.FORMAT_RGB8
	var img := Image.create_from_data(TEX_WIDTH, TEX_HEIGHT, false, fmt, bytes)
	return ImageTexture.create_from_image(img)
