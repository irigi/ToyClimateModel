extends Node3D
## toyclim viewer (SPEC.md §7): runs a case live through the ClimateModel GDExtension and
## shows one field on a globe or a flat map. The scene is built in code; the model logic
## lives in Rust, this script only moves images to textures and handles the UI.
##
## Command line (after `--`):
##   --case=<name in cases/ or path to .toml>   default: williamson6
##   --field=<name>  --level=<n>  --run  --sps=<model steps per second, 0 = max>
##   --map (start in the flat map view)
##   --screenshot=<png path> [--after=<seconds, default 3>]: save the window and quit
## In headless mode (`--headless`) the model runs at once and progress is printed every
## second, which is the M3 check: godot4 --headless --path godot --quit-after 600

const DEFAULT_CASE := "williamson6"
## First match is shown after loading a case.
const PREFERRED_FIELDS := ["h", "tas", "ts", "t850", "vor", "t"]
## Fields shown with a diverging colormap and a range symmetric about zero.
const SIGNED_FIELDS := ["vor", "div", "u", "v", "eta", "pv", "u850", "v850", "u250",
	"vor850", "uas", "vas", "omega", "q_flux", "surface_heat", "shf", "lhf"]
const SPEEDS := [0.0, 3000.0, 1000.0, 300.0, 100.0, 30.0, 10.0, 3.0, 1.0]

var model: Node
var repo_root: String
var headless := false
var args := {}

var field := ""
var level := 0
var known_fields := PackedStringArray()
var field_tex: ImageTexture
var globe_mat: ShaderMaterial
var map_mat: ShaderMaterial
var cmap_seq: GradientTexture1D
var cmap_div: GradientTexture1D
var vrange := Vector2(0, 1)

var pivot: Node3D
var camera: Camera3D
var globe: MeshInstance3D
var map_rect: TextureRect
var ui := {}
var dragging := false

var start_step := -1
var last_print_ms := 0


func _ready() -> void:
	headless = DisplayServer.get_name() == "headless"
	repo_root = ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	args = _parse_args()
	if not ClassDB.class_exists("ClimateModel"):
		push_error("toyclim extension not loaded; build it with ./run")
		get_tree().quit(1)
		return
	model = ClassDB.instantiate("ClimateModel")
	add_child(model)
	print("toyclim viewer: ", model.version(), ", Godot ", Engine.get_version_info().string)
	_build_scene()
	_build_ui()
	_load_case(args.get("case", DEFAULT_CASE))
	if args.has("sps"):
		model.set_steps_per_second(float(args["sps"]))
	if headless or args.has("run"):
		model.run()
	if args.has("map"):
		_toggle_view()
	if args.has("screenshot"):
		_screenshot(args["screenshot"], float(args.get("after", 3.0)))


func _screenshot(path: String, after: float) -> void:
	await get_tree().create_timer(after).timeout
	await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	var err := img.save_png(path)
	print("toyclim viewer: screenshot ", path, " (", error_string(err), ")")
	get_tree().quit(0 if err == OK else 1)


func _parse_args() -> Dictionary:
	var out := {}
	for a in OS.get_cmdline_user_args():
		var s: String = a.trim_prefix("--")
		var eq := s.find("=")
		if eq < 0:
			out[s] = true
		else:
			out[s.substr(0, eq)] = s.substr(eq + 1)
	return out


func _case_path(c: String) -> String:
	if c.is_absolute_path():
		return c
	if FileAccess.file_exists(repo_root.path_join(c)):
		return repo_root.path_join(c)
	return repo_root.path_join("cases").path_join(c.trim_suffix(".toml") + ".toml")


func _load_case(c: String) -> void:
	var path := _case_path(c)
	if not model.load_config(path, 0, 0):
		var err: String = model.get_last_error()
		push_error("cannot load %s: %s" % [path, err])
		ui.error.text = err
		if headless:
			get_tree().quit(1)
		return
	print("toyclim viewer: loaded ", path)
	ui.error.text = ""
	start_step = -1
	known_fields = PackedStringArray()
	field = ""
	level = int(args.get("level", 0))
	model.refresh()
	_sync_field_list()
	var want: String = args.get("field", "")
	if want == "" or not known_fields.has(want):
		want = known_fields[0] if known_fields.size() > 0 else ""
		for f in PREFERRED_FIELDS:
			if known_fields.has(f):
				want = f
				break
	_select_field(want)
	for i in ui.case.item_count:
		if _case_path(ui.case.get_item_text(i)) == path:
			ui.case.select(i)


# ---------------------------------------------------------------- scene

func _gradient(colors: Array) -> GradientTexture1D:
	var g := Gradient.new()
	var offsets := PackedFloat32Array()
	var cols := PackedColorArray()
	for i in colors.size():
		offsets.append(float(i) / (colors.size() - 1))
		cols.append(Color(colors[i]))
	g.offsets = offsets
	g.colors = cols
	var t := GradientTexture1D.new()
	t.gradient = g
	t.width = 256
	return t


func _build_scene() -> void:
	# viridis-like and RdBu-like
	cmap_seq = _gradient(["#440154", "#3b528b", "#21918c", "#5ec962", "#fde725"])
	cmap_div = _gradient(["#053061", "#4393c3", "#f7f7f7", "#d6604d", "#67001f"])

	var env := WorldEnvironment.new()
	env.environment = Environment.new()
	env.environment.background_mode = Environment.BG_COLOR
	env.environment.background_color = Color(0.06, 0.06, 0.08)
	add_child(env)

	globe_mat = ShaderMaterial.new()
	globe_mat.shader = load("res://shaders/field_globe.gdshader")
	var sphere := SphereMesh.new()
	sphere.radius = 1.0
	sphere.height = 2.0
	sphere.radial_segments = 128
	sphere.rings = 64
	globe = MeshInstance3D.new()
	if not headless:  # the dummy renderer of --headless complains about meshes
		globe.mesh = sphere
	globe.material_override = globe_mat
	add_child(globe)

	pivot = Node3D.new()
	add_child(pivot)
	camera = Camera3D.new()
	camera.position = Vector3(0, 0, 3.2)
	camera.fov = 40
	pivot.add_child(camera)
	camera.make_current()
	# Look at 0°E from above the equator, tilted a bit to show the north.
	pivot.rotation = Vector3(deg_to_rad(-20), 0, 0)

	map_mat = ShaderMaterial.new()
	map_mat.shader = load("res://shaders/field_map.gdshader")


func _build_ui() -> void:
	var layer := CanvasLayer.new()
	add_child(layer)

	map_rect = TextureRect.new()
	map_rect.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	map_rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	map_rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	map_rect.material = map_mat
	map_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	map_rect.visible = false
	layer.add_child(map_rect)

	var panel := PanelContainer.new()
	panel.position = Vector2(8, 8)
	layer.add_child(panel)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(260, 0)
	panel.add_child(box)

	ui.case = OptionButton.new()
	var files := DirAccess.get_files_at(repo_root.path_join("cases"))
	files.sort()
	for f in files:
		if f.ends_with(".toml"):
			ui.case.add_item(f.trim_suffix(".toml"))
	ui.case.item_selected.connect(func(i): _load_case(ui.case.get_item_text(i)))
	_row(box, "Case", ui.case)

	ui.field = OptionButton.new()
	ui.field.item_selected.connect(func(i): _select_field(ui.field.get_item_text(i)))
	_row(box, "Field", ui.field)

	ui.level = SpinBox.new()
	ui.level.min_value = 0
	ui.level.value_changed.connect(func(v): level = int(v); _show_field())
	ui.level_row = _row(box, "Level (0 = top)", ui.level)

	ui.auto = CheckBox.new()
	ui.auto.text = "auto range"
	ui.auto.button_pressed = true
	ui.auto.toggled.connect(func(_on): _show_field())
	box.add_child(ui.auto)
	var range_box := HBoxContainer.new()
	ui.vmin = _number()
	ui.vmax = _number()
	ui.vmin.text_submitted.connect(func(_t): _range_edited())
	ui.vmax.text_submitted.connect(func(_t): _range_edited())
	range_box.add_child(ui.vmin)
	range_box.add_child(ui.vmax)
	box.add_child(range_box)

	var buttons := HBoxContainer.new()
	ui.run = Button.new()
	ui.run.text = "Run"
	ui.run.custom_minimum_size = Vector2(70, 0)
	ui.run.pressed.connect(_toggle_run)
	buttons.add_child(ui.run)
	var step := Button.new()
	step.text = "Step"
	step.pressed.connect(func(): model.step(1))
	buttons.add_child(step)
	ui.view = Button.new()
	ui.view.text = "Map"
	ui.view.pressed.connect(_toggle_view)
	buttons.add_child(ui.view)
	box.add_child(buttons)

	ui.speed = OptionButton.new()
	for s in SPEEDS:
		ui.speed.add_item("max" if s == 0.0 else "%d steps/s" % s)
	ui.speed.item_selected.connect(func(i): model.set_steps_per_second(SPEEDS[i]))
	_row(box, "Speed", ui.speed)

	ui.info = Label.new()
	box.add_child(ui.info)
	ui.error = Label.new()
	ui.error.add_theme_color_override("font_color", Color(1, 0.4, 0.4))
	ui.error.autowrap_mode = TextServer.AUTOWRAP_WORD
	box.add_child(ui.error)
	var hint := Label.new()
	hint.text = "drag: rotate  wheel: zoom\nspace: run/pause  M: map/globe"
	hint.modulate = Color(1, 1, 1, 0.6)
	box.add_child(hint)


func _row(parent: Control, label: String, c: Control) -> Control:
	var row := VBoxContainer.new()
	var l := Label.new()
	l.text = label
	row.add_child(l)
	row.add_child(c)
	parent.add_child(row)
	return row


func _number() -> LineEdit:
	var e := LineEdit.new()
	e.custom_minimum_size = Vector2(125, 0)
	e.tooltip_text = "colour range; Enter applies and switches auto range off"
	return e


# ---------------------------------------------------------------- field display

func _sync_field_list() -> void:
	var avail: PackedStringArray = model.get_available_fields()
	if avail == known_fields:
		return
	known_fields = avail
	ui.field.clear()
	for f in avail:
		ui.field.add_item(f)
		if f == field:
			ui.field.select(ui.field.item_count - 1)


func _select_field(f: String) -> void:
	if f == "":
		return
	if f != field:
		field = f
		ui.auto.button_pressed = true
	for i in ui.field.item_count:
		if ui.field.get_item_text(i) == f:
			ui.field.select(i)
	model.set_fields(PackedStringArray([f]))
	var cmap := cmap_div if SIGNED_FIELDS.has(f) else cmap_seq
	globe_mat.set_shader_parameter("colormap", cmap)
	map_mat.set_shader_parameter("colormap", cmap)


func _show_field() -> void:
	var n: int = model.get_level_count(field)
	if n == 0:
		return  # the snapshot with this field has not arrived yet
	ui.level_row.visible = n > 1
	ui.level.max_value = n - 1
	level = clampi(level, 0, n - 1)
	ui.level.set_value_no_signal(level)
	var img: Image = model.get_field_image(field, level)
	if img == null:
		return
	if field_tex == null or field_tex.get_width() != img.get_width() \
			or field_tex.get_height() != img.get_height():
		field_tex = ImageTexture.create_from_image(img)
		globe_mat.set_shader_parameter("field", field_tex)
		map_mat.set_shader_parameter("field", field_tex)
		map_rect.texture = field_tex
	else:
		field_tex.update(img)
	if ui.auto.button_pressed:
		var r: Vector2 = model.get_field_range(field, level)
		if SIGNED_FIELDS.has(field):
			var m := maxf(absf(r.x), absf(r.y))
			r = Vector2(-m, m)
		vrange = r
		ui.vmin.text = _num(r.x)
		ui.vmax.text = _num(r.y)
	_apply_range()


func _range_edited() -> void:
	if ui.vmin.text.is_valid_float() and ui.vmax.text.is_valid_float():
		vrange = Vector2(ui.vmin.text.to_float(), ui.vmax.text.to_float())
		ui.auto.set_pressed_no_signal(false)
		_apply_range()


func _apply_range() -> void:
	for m in [globe_mat, map_mat]:
		m.set_shader_parameter("vmin", vrange.x)
		m.set_shader_parameter("vmax", vrange.y)


func _toggle_run() -> void:
	if model.is_running():
		model.pause()
	else:
		model.run()


func _toggle_view() -> void:
	map_rect.visible = not map_rect.visible
	globe.visible = not map_rect.visible
	ui.view.text = "Globe" if map_rect.visible else "Map"


# ---------------------------------------------------------------- frame loop

func _process(_dt: float) -> void:
	if model == null or not model.refresh():
		return
	_sync_field_list()
	_show_field()
	var step: int = model.get_step()
	if start_step < 0:
		start_step = step
	var running: bool = model.is_running()
	ui.run.text = "Pause" if running else "Run"
	var lines := ["day %.3f   step %d" % [model.get_time_days(), step]]
	if running:
		lines.append("%.0f steps/s" % model.get_steps_per_second())
	var d: Dictionary = model.get_diagnostics()
	for k in d:
		lines.append("%s: %s" % [k, _num(d[k])])
	ui.info.text = "\n".join(lines)
	var err: String = model.get_model_error()
	if err != "":
		ui.error.text = err
	if headless and Time.get_ticks_msec() - last_print_ms >= 1000:
		last_print_ms = Time.get_ticks_msec()
		print("toyclim: step %d  day %.4f  %.0f steps/s  %s" % [step, model.get_time_days(),
				model.get_steps_per_second(), err])


## Compact number formatting (GDScript's % operator has no %g).
func _num(x: float) -> String:
	var a := absf(x)
	if a == 0.0 or (a >= 1e-2 and a < 1e6):
		return String.num(x, 4)
	return "%.4fe%d" % [x / pow(10.0, floorf(log(a) / log(10.0))), int(floorf(log(a) / log(10.0)))]


func _exit_tree() -> void:
	if model != null and headless:
		model.refresh()
		var n: int = model.get_step() - maxi(start_step, 0)
		print("toyclim: finished at step %d, day %.4f (advanced %d steps)%s" % [
				model.get_step(), model.get_time_days(), n,
				"" if n > 0 else "  -- MODEL DID NOT ADVANCE"])


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		if event.keycode == KEY_SPACE:
			_toggle_run()
		elif event.keycode == KEY_M:
			_toggle_view()
	if map_rect == null or map_rect.visible:
		return
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			dragging = event.pressed
		elif event.button_index == MOUSE_BUTTON_WHEEL_UP and event.pressed:
			camera.position.z = maxf(1.3, camera.position.z * 0.9)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN and event.pressed:
			camera.position.z = minf(10.0, camera.position.z * 1.1)
	elif event is InputEventMouseMotion and dragging:
		var k := 0.005 * (camera.position.z - 1.0) / 2.2
		pivot.rotation.y -= event.relative.x * k
		pivot.rotation.x = clampf(pivot.rotation.x - event.relative.y * k, -1.5, 1.5)
