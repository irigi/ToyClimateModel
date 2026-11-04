extends Node3D
# M0 placeholder: proves the extension loads. The viewer arrives in M3.

func _ready() -> void:
	if ClassDB.class_exists("ClimateModel"):
		var model = ClassDB.instantiate("ClimateModel")
		add_child(model)
		print("loaded ", model.version())
	else:
		push_error("toyclim extension not loaded; run ./run to build it")
