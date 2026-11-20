extends Node3D
## Genesis graphics-only benchmark. Real gameplay lives in Rust.
## A JSON recording from src/bin/scene_export.rs is replayed without simulation
## authority. Without that recording, the scene says VISUAL PREVIEW.

const TARGET_FPS := 90.0
const FRAME_BUDGET_MS := 1000.0 / TARGET_FPS
const REPLAY_TICKS_PER_SECOND := 14.0
const FORMATION_SOLDIERS := 48
const PRESET_SCALES := [1.0, 0.85, 0.70]
const FOREST_LIMIT := 950

var data: Dictionary = {}
var frames: Array = []
var map_w := 72
var map_h := 52
var preview_only := true
var noise := FastNoiseLite.new()
var player_meshes: Array[MultiMeshInstance3D] = []
var banner_poles: Array[MeshInstance3D] = []
var cam: Camera3D
var hud: Label
var playhead := 0.0
var active := true

var camera_center := Vector3.ZERO
var desired_center := Vector3.ZERO
var camera_yaw := 0.55
var camera_pitch := 0.75
var camera_distance := 68.0
var desired_distance := 68.0
var touches: Dictionary = {}
var previous_pinch := 0.0

var frame_times: Array[float] = []
var refresh_timer := 0.0
var slow_windows := 0
var fast_windows := 0
var quality_idx := 0
var display_hz := 60.0

func _ready() -> void:
	Engine.max_fps = 0 # Do not cap a 90/120 Hz display at 60.
	display_hz = DisplayServer.screen_get_refresh_rate()
	get_viewport().scaling_3d_scale = 1.0
	_load_recording()
	noise.seed = int(data.get("seed", 77001))
	noise.frequency = 0.055
	_create_terrain()
	_create_water()
	_create_forest()
	_create_armies()
	_create_environment()
	_create_camera()
	_create_hud()
	set_process(true)
	set_process_input(true)

func _load_recording() -> void:
	const PATH := "res://scene_data/frontier_demo.json"
	if FileAccess.file_exists(PATH):
		var file := FileAccess.open(PATH, FileAccess.READ)
		var parsed: Variant = JSON.parse_string(file.get_as_text())
		if parsed is Dictionary and parsed.get("source", "") == "rust-frontier" and int(parsed.get("schema_version", 0)) == 1:
			data = parsed
			preview_only = false
	if data.is_empty():
		data = _make_preview()
		preview_only = true
	map_w = int(data.get("width", 72))
	map_h = int(data.get("height", 52))
	frames = data.get("frames", [])

func _make_preview() -> Dictionary:
	# This is deliberately NOT called simulation: a fallback for opening
	# the Godot project before recording Rust world states with Cargo.
	var surfaces: Array = []
	for z in range(52):
		for x in range(72):
			var band := sin(float(x) * 0.17) + cos(float(z) * 0.31)
			var surface := 1
			if abs(float(x) - 35.0 - sin(float(z) * 0.19) * 3.0) < 2.0:
				surface = 0
			elif band > 1.2:
				surface = 4
			elif sin(float(x) * 0.34 + float(z) * 0.2) > 0.48:
				surface = 3
			elif band < -0.6:
				surface = 2
			surfaces.append(surface)
	var demo_frames: Array = []
	for step in range(421):
		var x_a := 11.0 + min(float(step) / 16.0, 25.0)
		var x_b := 60.0 - min(float(step) / 16.0, 25.0)
		demo_frames.append({
			"tick": step,
			"armies": [[1, 0, x_a, 18.0, 300], [2, 1, x_b, 22.0, 300]]
		})
	return {"width": 72, "height": 52, "seed": 77001, "surface": surfaces, "frames": demo_frames}

func _tile(x: int, z: int) -> int:
	var tiles: Array = data.get("surface", [])
	var index := clampi(z, 0, map_h - 1) * map_w + clampi(x, 0, map_w - 1)
	if index >= 0 and index < tiles.size():
		return int(tiles[index])
	return 1

func _height(gx: float, gz: float) -> float:
	var x := clampi(int(floor(gx)), 0, map_w - 1)
	var z := clampi(int(floor(gz)), 0, map_h - 1)
	var terrain := _tile(x, z)
	var n := noise.get_noise_2d(gx, gz)
	if terrain == 0:
		return 0.05 + n * 0.04
	var elevation := 0.90 + n * 0.48
	if terrain == 4:
		elevation += 1.65
	return elevation

func _point(gx: float, gz: float) -> Vector3:
	return Vector3(gx - float(map_w) * 0.5, _height(gx, gz), gz - float(map_h) * 0.5)

func _surface_color(terrain: int, x: int, z: int) -> Color:
	var shade := noise.get_noise_2d(float(x) * 2.7, float(z) * 2.7) * 0.055
	match terrain:
		0: return Color(0.13 + shade, 0.33 + shade, 0.41 + shade)
		2: return Color(0.47 + shade, 0.54 + shade, 0.29 + shade)
		3: return Color(0.21 + shade, 0.37 + shade, 0.27 + shade)
		4: return Color(0.42 + shade, 0.43 + shade, 0.41 + shade)
		_: return Color(0.35 + shade, 0.46 + shade, 0.31 + shade)

func _vertex(tool: SurfaceTool, point: Vector3, color: Color) -> void:
	tool.set_color(color)
	tool.add_vertex(point)

func _create_terrain() -> void:
	# One combined ~30k triangle surface, no node per tile.
	var tool := SurfaceTool.new()
	tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	for z in range(map_h):
		for x in range(map_w):
			var color := _surface_color(_tile(x, z), x, z)
			for sub_z in range(2):
				for sub_x in range(2):
					var gx := float(x) + float(sub_x) * 0.5
					var gz := float(z) + float(sub_z) * 0.5
					var a := _point(gx, gz)
					var b := _point(gx + 0.5, gz)
					var c := _point(gx, gz + 0.5)
					var d := _point(gx + 0.5, gz + 0.5)
					_vertex(tool, a, color)
					_vertex(tool, d, color)
					_vertex(tool, b, color)
					_vertex(tool, a, color)
					_vertex(tool, c, color)
					_vertex(tool, d, color)
	tool.generate_normals()
	var land := MeshInstance3D.new()
	land.name = "BatchedTerrain"
	land.mesh = tool.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.95
	land.material_override = mat
	land.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(land)

func _create_water() -> void:
	var water := MeshInstance3D.new()
	water.name = "Water"
	var plane := PlaneMesh.new()
	plane.size = Vector2(float(map_w), float(map_h))
	water.mesh = plane
	water.position.y = 0.28
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.11, 0.31, 0.41)
	mat.roughness = 0.34
	mat.metallic = 0.12
	water.material_override = mat
	water.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(water)

func _create_forest() -> void:
	# Two GPU instancing batches for every tree: foliage and trunk.
	var positions: Array[Vector3] = []
	var rng := RandomNumberGenerator.new()
	rng.seed = int(data.get("seed", 77001))
	for z in range(map_h):
		for x in range(map_w):
			if _tile(x, z) != 3:
				continue
			var count := 1 + (1 if rng.randf() > 0.60 else 0)
			for i in range(count):
				if positions.size() >= FOREST_LIMIT:
					break
				var gx := float(x) + rng.randf_range(0.20, 0.80)
				var gz := float(z) + rng.randf_range(0.20, 0.80)
				positions.append(_point(gx, gz))
	if positions.is_empty():
		return
	var crown := CylinderMesh.new()
	crown.top_radius = 0.0
	crown.bottom_radius = 0.60
	crown.height = 2.40
	crown.radial_segments = 5
	var crown_mat := StandardMaterial3D.new()
	crown_mat.albedo_color = Color(0.17, 0.32, 0.23)
	crown_mat.roughness = 1.0
	crown.material = crown_mat
	var trunk := CylinderMesh.new()
	trunk.top_radius = 0.09
	trunk.bottom_radius = 0.13
	trunk.height = 1.0
	trunk.radial_segments = 5
	var trunk_mat := StandardMaterial3D.new()
	trunk_mat.albedo_color = Color(0.33, 0.27, 0.19)
	trunk.material = trunk_mat
	var forest := MultiMesh.new()
	forest.transform_format = MultiMesh.TRANSFORM_3D
	forest.mesh = crown
	forest.instance_count = positions.size()
	var woods := MultiMesh.new()
	woods.transform_format = MultiMesh.TRANSFORM_3D
	woods.mesh = trunk
	woods.instance_count = positions.size()
	for i in range(positions.size()):
		var height_scale := rng.randf_range(0.75, 1.55)
		var scale_basis := Basis().scaled(Vector3(height_scale, height_scale, height_scale))
		forest.set_instance_transform(i, Transform3D(scale_basis, positions[i] + Vector3.UP * 1.75 * height_scale))
		woods.set_instance_transform(i, Transform3D(Basis(), positions[i] + Vector3.UP * 0.4))
	var crowns := MultiMeshInstance3D.new()
	crowns.name = "InstancedFoliage"
	crowns.multimesh = forest
	crowns.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(crowns)
	var trunks := MultiMeshInstance3D.new()
	trunks.name = "InstancedTrunks"
	trunks.multimesh = woods
	trunks.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(trunks)

func _create_armies() -> void:
	for side in range(2):
		var capsule := CapsuleMesh.new()
		capsule.radius = 0.17
		capsule.height = 0.91
		var mat := StandardMaterial3D.new()
		mat.albedo_color = Color(0.16, 0.76, 0.97) if side == 0 else Color(1.0, 0.43, 0.22)
		mat.roughness = 0.65
		capsule.material = mat
		var batch := MultiMesh.new()
		batch.transform_format = MultiMesh.TRANSFORM_3D
		batch.mesh = capsule
		batch.instance_count = FORMATION_SOLDIERS
		var soldiers := MultiMeshInstance3D.new()
		soldiers.name = "Army_%d" % side
		soldiers.multimesh = batch
		soldiers.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(soldiers)
		player_meshes.append(soldiers)

		var pole := MeshInstance3D.new()
		var pole_geo := BoxMesh.new()
		pole_geo.size = Vector3(0.09, 3.5, 0.09)
		pole.mesh = pole_geo
		add_child(pole)
		banner_poles.append(pole)

func _create_environment() -> void:
	var env := WorldEnvironment.new()
	var lighting := Environment.new()
	lighting.background_mode = Environment.BG_COLOR
	lighting.background_color = Color(0.46, 0.61, 0.68)
	lighting.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	lighting.ambient_light_color = Color(0.62, 0.69, 0.75)
	lighting.ambient_light_energy = 0.70
	lighting.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.environment = lighting
	add_child(env)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-56, 24, 0)
	sun.light_energy = 1.35
	sun.shadow_enabled = false # Enable after profiling device headroom.
	add_child(sun)

func _create_camera() -> void:
	cam = Camera3D.new()
	cam.name = "OrbitCamera"
	cam.fov = 55.0
	cam.near = 0.08
	cam.far = 350.0
	add_child(cam)
	cam.current = true
	_update_camera(0.016)

func _create_hud() -> void:
	var layer := CanvasLayer.new()
	add_child(layer)
	hud = Label.new()
	hud.position = Vector2(22, 20)
	hud.add_theme_font_size_override("font_size", 18)
	hud.add_theme_color_override("font_color", Color(0.94, 0.98, 0.97))
	hud.add_theme_color_override("font_shadow_color", Color(0.03, 0.08, 0.10, 0.95))
	hud.add_theme_constant_override("shadow_offset_x", 2)
	hud.add_theme_constant_override("shadow_offset_y", 2)
	layer.add_child(hud)

func _army_at(frame: Dictionary, side: int) -> Vector3:
	for record in frame.get("armies", []):
		if int(record[1]) == side:
			return _point(float(record[2]) + 0.5, float(record[3]) + 0.5)
	return Vector3(0, -1000, 0)

func _update_armies() -> void:
	if frames.is_empty():
		return
	var current := clampi(int(floor(playhead)), 0, frames.size() - 1)
	var next := min(current + 1, frames.size() - 1)
	var t := playhead - float(current)
	for side in range(2):
		var a := _army_at(frames[current], side)
		var b := _army_at(frames[next], side)
		var alive := a.y > -500 and b.y > -500
		player_meshes[side].visible = alive
		banner_poles[side].visible = alive
		if not alive:
			continue
		var center := a.lerp(b, t)
		var direction := b - a
		var heading := atan2(direction.x, direction.z) if direction.length_squared() > 0.00001 else 0.0
		var basis := Basis(Vector3.UP, heading)
		var multimesh := player_meshes[side].multimesh
		for i in range(FORMATION_SOLDIERS):
			var col := i % 8
			var row := i / 8
			var offset := Vector3((float(col) - 3.5) * 0.55, 0.0, (float(row) - 2.5) * 0.69)
			var p := center + basis * offset
			p.y = _height(p.x + float(map_w) * 0.5, p.z + float(map_h) * 0.5) + 0.58
			multimesh.set_instance_transform(i, Transform3D(basis, p))
		banner_poles[side].position = center + Vector3(0, 2.6, 0)

func _update_camera(delta: float) -> void:
	var smoothing := 1.0 - exp(-delta * 9.0)
	camera_center = camera_center.lerp(desired_center, smoothing)
	camera_distance = lerpf(camera_distance, desired_distance, smoothing)
	var outward := Vector3(
		sin(camera_yaw) * cos(camera_pitch),
		sin(camera_pitch),
		cos(camera_yaw) * cos(camera_pitch)
	)
	cam.position = camera_center + outward * camera_distance
	cam.look_at(camera_center, Vector3.UP)

func _process(delta: float) -> void:
	var dt := minf(delta, 0.12)
	if active and frames.size() > 1:
		playhead += dt * REPLAY_TICKS_PER_SECOND
		if playhead >= float(frames.size() - 1):
			playhead = 0.0
	_update_armies()
	_update_camera(dt)
	frame_times.append(delta * 1000.0)
	if frame_times.size() > 180:
		frame_times.pop_front()
	refresh_timer += dt
	if refresh_timer >= 1.0:
		refresh_timer = 0.0
		_update_performance()

func _update_performance() -> void:
	if frame_times.is_empty():
		return
	var ordered := frame_times.duplicate()
	ordered.sort()
	var p95 := ordered[clampi(int(ceil(float(ordered.size()) * 0.95)) - 1, 0, ordered.size() - 1)]
	var fps := Engine.get_frames_per_second()
	var supports_90 := display_hz <= 0.0 or display_hz >= 85.0
	# Never reduce resolution to "fix" a physical 60 Hz refresh limit.
	if supports_90:
		if p95 > FRAME_BUDGET_MS * 1.20:
			slow_windows += 1
			fast_windows = 0
		elif p95 < FRAME_BUDGET_MS * 0.82:
			fast_windows += 1
			slow_windows = 0
		else:
			slow_windows = 0
			fast_windows = 0
		if slow_windows >= 3 and quality_idx < PRESET_SCALES.size() - 1:
			quality_idx += 1
			slow_windows = 0
		if fast_windows >= 5 and quality_idx > 0:
			quality_idx -= 1
			fast_windows = 0
		get_viewport().scaling_3d_scale = PRESET_SCALES[quality_idx]
	var mode := "VISUAL PREVIEW (NO RUST SNAPSHOT)" if preview_only else "RUST FRONTIER REPLAY — READ ONLY"
	var screen_note := "DISPLAY %.0f Hz" % display_hz if display_hz > 0.0 and display_hz < 85.0 else "TARGET 90 FPS"
	hud.text = "GENESIS / 3D GRAPHICS BENCHMARK\n%s\n%.0f FPS   |   p95 %.1f ms   |   budget %.1f ms\n%s   |   render %.0f%%   |   tick %d\nDrag: pan   Right drag: orbit   Wheel/pinch: zoom   Q: quality   Space: pause" % [
		mode, fps, p95, FRAME_BUDGET_MS, screen_note,
		PRESET_SCALES[quality_idx] * 100.0, int(frames[clampi(int(playhead), 0, frames.size()-1)].get("tick", 0))
	]

func _pan(delta_pixels: Vector2) -> void:
	var scale := camera_distance * 0.0017
	var right := Vector3(cos(camera_yaw), 0, -sin(camera_yaw))
	var forward := Vector3(sin(camera_yaw), 0, cos(camera_yaw))
	desired_center -= right * delta_pixels.x * scale
	desired_center -= forward * delta_pixels.y * scale
	desired_center.x = clampf(desired_center.x, -float(map_w) * 0.5, float(map_w) * 0.5)
	desired_center.z = clampf(desired_center.z, -float(map_h) * 0.5, float(map_h) * 0.5)

func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		if event.keycode == KEY_SPACE:
			active = not active
		elif event.keycode == KEY_Q:
			quality_idx = (quality_idx + 1) % PRESET_SCALES.size()
			get_viewport().scaling_3d_scale = PRESET_SCALES[quality_idx]
	if event is InputEventMouseButton and event.pressed:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			desired_distance = maxf(10.0, desired_distance * 0.89)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			desired_distance = minf(120.0, desired_distance * 1.12)
	if event is InputEventMouseMotion:
		if Input.is_mouse_button_pressed(MOUSE_BUTTON_RIGHT):
			camera_yaw -= event.relative.x * 0.005
			camera_pitch = clampf(camera_pitch - event.relative.y * 0.004, 0.30, 1.30)
		elif Input.is_mouse_button_pressed(MOUSE_BUTTON_LEFT):
			_pan(event.relative)
	if event is InputEventScreenTouch:
		if event.pressed:
			touches[event.index] = event.position
		else:
			touches.erase(event.index)
		previous_pinch = 0.0
	if event is InputEventScreenDrag:
		touches[event.index] = event.position
		if touches.size() == 1:
			_pan(event.relative)
		elif touches.size() == 2:
			var keys := touches.keys()
			var distance := (touches[keys[0]] as Vector2).distance_to(touches[keys[1]] as Vector2)
			if previous_pinch > 1.0:
				desired_distance = clampf(desired_distance * previous_pinch / maxf(1.0, distance), 10.0, 120.0)
			previous_pinch = distance
