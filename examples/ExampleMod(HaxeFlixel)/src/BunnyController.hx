package;

import hashloader.HashLoader;

/**
 * Controlador de efectos visuales y físicos sobre el grupo `_bunnies` de `PlayState`.
 * Usa acceso directo al `hl.NativeArray` subyacente y actualización escalonada de color
 * para mantener 60 FPS estables incluso con miles de conejos en pantalla.
 */
class BunnyController {
	public var rainbowEnabled:Bool = true;
	/** 0 = Normal (y=+5), 1 = Invertida (y=-180), 2 = Cero-G (y=0) */
	public var gravityMode:Int = 0;

	var frameTick:Int = 0;
	var phase:Float = 0.0;
	var lastAppliedCount:Int = -1;
	var lastAppliedGravity:Int = -1;

	var palette:Array<Int> = [
		0xFFFF5252, // Rojo coral
		0xFFFFAB40, // Naranja
		0xFFFFEA00, // Amarillo
		0xFF69F0AE, // Verde menta
		0xFF40C4FF, // Celeste
		0xFF7C4DFF, // Violeta
		0xFFFF4081  // Rosa neón
	];

	public function new() {}

	public function toggleRainbow(state:Dynamic) {
		rainbowEnabled = !rainbowEnabled;
		if (!rainbowEnabled) {
			var membersObj = getMembersObj(state);
			if (membersObj == null) return;
			var len = HashLoader.arrayLength(membersObj);
			var raw:hl.NativeArray<Dynamic> = HashLoader.field(membersObj, "array");
			if (raw == null) return;
			for (i in 0...len) {
				var b:Dynamic = raw[i];
				if (b != null) {
					HashLoader.call(b, "set_color", [0xFFFFFFFF]);
				}
			}
		}
	}

	public function cycleGravity(state:Dynamic) {
		gravityMode = (gravityMode + 1) % 3;
		lastAppliedGravity = -1; // forzar aplicación en el próximo update

		var membersObj = getMembersObj(state);
		if (membersObj == null) return;
		var len = HashLoader.arrayLength(membersObj);
		var raw:hl.NativeArray<Dynamic> = HashLoader.field(membersObj, "array");
		if (raw == null) return;

		for (i in 0...len) {
			var b:Dynamic = raw[i];
			if (b == null) continue;
			var vel:Dynamic = HashLoader.field(b, "velocity");
			if (vel != null) {
				var vy:Float = HashLoader.field(vel, "y");
				if (gravityMode == 1 && vy > -40.0) {
					HashLoader.setField(vel, "y", -160.0);
				} else if (gravityMode == 0 && vy < 40.0) {
					HashLoader.setField(vel, "y", 160.0);
				}
			}
		}
	}

	/** Impulso explosivo radial a todos los conejos. */
	public function triggerBurst(state:Dynamic) {
		var membersObj = getMembersObj(state);
		if (membersObj == null) return;
		var len = HashLoader.arrayLength(membersObj);
		var raw:hl.NativeArray<Dynamic> = HashLoader.field(membersObj, "array");
		if (raw == null) return;

		var cx:Float = 320.0;
		var cy:Float = 240.0;
		for (i in 0...len) {
			var b:Dynamic = raw[i];
			if (b == null) continue;
			var vel:Dynamic = HashLoader.field(b, "velocity");
			if (vel != null) {
				var bx:Float = HashLoader.field(b, "x");
				var by:Float = HashLoader.field(b, "y");
				var dx = bx - cx;
				var dy = by - cy;
				var dist = Math.sqrt(dx * dx + dy * dy);
				if (dist < 1.0) {
					var ang = (i * 0.61803398875) * Math.PI * 2.0;
					dx = Math.cos(ang);
					dy = Math.sin(ang);
					dist = 1.0;
				}
				var speed = 280.0 + (i % 7) * 35.0;
				HashLoader.setField(vel, "x", (dx / dist) * speed);
				HashLoader.setField(vel, "y", (dy / dist) * speed);
			}
		}
	}

	public function update(state:Dynamic, mouseMode:Int, mx:Float, my:Float) {
		frameTick++;
		phase += 0.08;

		var membersObj = getMembersObj(state);
		if (membersObj == null) return;
		var len = HashLoader.arrayLength(membersObj);
		if (len <= 0) return;

		var raw:hl.NativeArray<Dynamic> = HashLoader.field(membersObj, "array");
		if (raw == null) return;

		// 1. Actualizar gravedad solo cuando cambia el modo o la cantidad de conejos
		if (gravityMode != lastAppliedGravity || len != lastAppliedCount) {
			lastAppliedGravity = gravityMode;
			lastAppliedCount = len;
			var targetAccY:Float = switch (gravityMode) {
				case 1: -180.0;
				case 2: 0.0;
				default: 5.0;
			};
			for (i in 0...len) {
				var b:Dynamic = raw[i];
				if (b == null) continue;
				var acc:Dynamic = HashLoader.field(b, "acceleration");
				if (acc != null) {
					HashLoader.setField(acc, "y", targetAccY);
				}
			}
		}

		// 2. Efecto arcoíris escalonado (actualiza 1/8 de los conejos por frame para mantener 60 FPS)
		if (rainbowEnabled) {
			var palLen = palette.length;
			var stride = len > 400 ? 8 : 2;
			var idx = frameTick % stride;
			while (idx < len) {
				var b:Dynamic = raw[idx];
				if (b != null) {
					var colorIdx = Std.int(Math.abs(Math.floor(phase + idx * 0.15))) % palLen;
					var wantColor = palette[colorIdx];
					var curColor:Int = HashLoader.field(b, "color");
					if (curColor != wantColor) {
						HashLoader.call(b, "set_color", [wantColor]);
					}
				}
				idx += stride;
			}
		}

		// 3. Interacción con el ratón (1 = Repeler, 2 = Atraer)
		if (mouseMode != 0) {
			var radius = 160.0;
			var radiusSq = radius * radius;
			var dir = (mouseMode == 1) ? 1.0 : -1.0;
			for (i in 0...len) {
				var b:Dynamic = raw[i];
				if (b == null) continue;
				var bx:Float = HashLoader.field(b, "x");
				var by:Float = HashLoader.field(b, "y");
				var dx = bx - mx;
				var dy = by - my;
				var distSq = dx * dx + dy * dy;
				if (distSq < radiusSq && distSq > 1.0) {
					var dist = Math.sqrt(distSq);
					var factor = (1.0 - dist / radius) * 28.0 * dir;
					var vel:Dynamic = HashLoader.field(b, "velocity");
					if (vel != null) {
						var vx:Float = HashLoader.field(vel, "x");
						var vy:Float = HashLoader.field(vel, "y");
						HashLoader.setField(vel, "x", vx + (dx / dist) * factor);
						HashLoader.setField(vel, "y", vy + (dy / dist) * factor);
					}
				}
			}
		}
	}

	static function getMembersObj(state:Dynamic):Dynamic {
		if (state == null) return null;
		var group:Dynamic = HashLoader.field(state, "_bunnies");
		if (group == null) return null;
		return HashLoader.field(group, "members");
	}
}
