extends Node3D

# Render resolution = WIDTH * 3 by HEIGHT * 3 (see planet_renderer.rs constants)
const TEX_WIDTH  := 512 * 3
const TEX_HEIGHT := 256 * 3
const ORBITAL_PERIOD := 536  # Oros days per year

var renderer: PlanetMapRenderer
var tick: int = 0
var update_interval: int

func _ready() -> void:
	renderer = PlanetMapRenderer.new()
	add_child(renderer)
	renderer.initialize()
	update_interval = renderer.update_interval_ticks()
	_apply_texture(renderer.annual_texture())

func _process(_delta: float) -> void:
	tick += 1
	if tick % update_interval == 0:
		var phase := float(tick % ORBITAL_PERIOD) / float(ORBITAL_PERIOD)
		_apply_texture(renderer.seasonal_texture(phase))

func _apply_texture(bytes: PackedByteArray) -> void:
	var img := Image.create_from_data(
		TEX_WIDTH, TEX_HEIGHT, false, Image.FORMAT_RGB8, bytes
	)
	var tex := ImageTexture.create_from_image(img)
	var mesh: MeshInstance3D = $MeshInstance3D
	var mat := mesh.get_surface_override_material(0)
	if mat == null:
		mat = StandardMaterial3D.new()
		mesh.set_surface_override_material(0, mat)
	(mat as StandardMaterial3D).albedo_texture = tex
