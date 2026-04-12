local M = {}

local k_button_b
local k_button_a
local clamp
local is_pilot
local is_telemetry
local is_moving
local read_transition
local next_scene
local prev_scene
local apply_transition
local transition_delta
local bump_pilot_frames
local bump_telemetry_frames
local read_direction
local read_side
local scene_title
local direction_label
local side_label
local transition_label
local scene_hint
local session_label
local engagement_label
local energy_delta
local heat_delta
local next_combo
local score_delta
local lap_delta
local alert_from_heat
local alert_label
local energy_label
local heat_label
local score_label
local combo_label
local lap_label
local progress_width
local direction_pointer_y
local pulse_line_y
local build_view
local init
local update
local render
local ViewModel_heading

k_button_b = function()
    return 16
end

k_button_a = function()
    return 32
end

clamp = function(v, lo, hi)
    return (function() if (v < lo) then return lo elseif (v > hi) then return hi else return v end end)()
end

is_pilot = function(scene)
    return (function(__scrutinee) if __scrutinee.tag == "Pilot" then return true elseif __scrutinee.tag == "Splash" then return false elseif __scrutinee.tag == "Telemetry" then return false else error("non-exhaustive match") end end)(scene)
end

is_telemetry = function(scene)
    return (function(__scrutinee) if __scrutinee.tag == "Telemetry" then return true elseif __scrutinee.tag == "Splash" then return false elseif __scrutinee.tag == "Pilot" then return false else error("non-exhaustive match") end end)(scene)
end

is_moving = function(direction)
    return (function(__scrutinee) if __scrutinee.tag == "Clockwise" then return true elseif __scrutinee.tag == "CounterClockwise" then return true elseif __scrutinee.tag == "Still" then return false else error("non-exhaustive match") end end)(direction)
end

read_transition = function()
    return (function() if playdate.buttonJustPressed(k_button_a()) then return { tag = "Advanced" } elseif playdate.buttonJustPressed(k_button_b()) then return { tag = "Rewound" } else return { tag = "Stayed" } end end)()
end

next_scene = function(scene)
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return { tag = "Pilot" } elseif __scrutinee.tag == "Pilot" then return { tag = "Telemetry" } elseif __scrutinee.tag == "Telemetry" then return { tag = "Splash" } else error("non-exhaustive match") end end)(scene)
end

prev_scene = function(scene)
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return { tag = "Telemetry" } elseif __scrutinee.tag == "Pilot" then return { tag = "Splash" } elseif __scrutinee.tag == "Telemetry" then return { tag = "Pilot" } else error("non-exhaustive match") end end)(scene)
end

apply_transition = function(scene, transition)
    return (function(__scrutinee) if __scrutinee.tag == "Advanced" then return next_scene(scene) elseif __scrutinee.tag == "Rewound" then return prev_scene(scene) elseif __scrutinee.tag == "Stayed" then return scene else error("non-exhaustive match") end end)(transition)
end

transition_delta = function(transition)
    return (function(__scrutinee) if __scrutinee.tag == "Stayed" then return 0 elseif __scrutinee.tag == "Advanced" then return 1 elseif __scrutinee.tag == "Rewound" then return 1 else error("non-exhaustive match") end end)(transition)
end

bump_pilot_frames = function(scene, total)
    return (function(__scrutinee) if __scrutinee.tag == "Pilot" then return (total + 1) elseif __scrutinee.tag == "Splash" then return total elseif __scrutinee.tag == "Telemetry" then return total else error("non-exhaustive match") end end)(scene)
end

bump_telemetry_frames = function(scene, total)
    return (function(__scrutinee) if __scrutinee.tag == "Telemetry" then return (total + 1) elseif __scrutinee.tag == "Splash" then return total elseif __scrutinee.tag == "Pilot" then return total else error("non-exhaustive match") end end)(scene)
end

read_direction = function(delta)
    return (function() if (delta > 0.0) then return { tag = "Clockwise" } elseif (delta < 0.0) then return { tag = "CounterClockwise" } else return { tag = "Still" } end end)()
end

read_side = function(position)
    return (function() if (position >= 180.0) then return { tag = "Right" } else return { tag = "Left" } end end)()
end

scene_title = function(scene)
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return "Scene: Splash" elseif __scrutinee.tag == "Pilot" then return "Scene: Pilot" elseif __scrutinee.tag == "Telemetry" then return "Scene: Telemetry" else error("non-exhaustive match") end end)(scene)
end

direction_label = function(direction)
    return (function(__scrutinee) if __scrutinee.tag == "Clockwise" then return "Crank: +" elseif __scrutinee.tag == "CounterClockwise" then return "Crank: -" elseif __scrutinee.tag == "Still" then return "Crank: 0" else error("non-exhaustive match") end end)(direction)
end

side_label = function(side)
    return (function(__scrutinee) if __scrutinee.tag == "Right" then return "Crank side: right" elseif __scrutinee.tag == "Left" then return "Crank side: left" else error("non-exhaustive match") end end)(side)
end

transition_label = function(transition)
    return (function(__scrutinee) if __scrutinee.tag == "Stayed" then return "Last transition: none" elseif __scrutinee.tag == "Advanced" then return "Last transition: next (A)" elseif __scrutinee.tag == "Rewound" then return "Last transition: prev (B)" else error("non-exhaustive match") end end)(transition)
end

scene_hint = function(scene)
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return "Press A: next scene, B: previous scene" elseif __scrutinee.tag == "Pilot" then return "Spin crank for score, combo, and lap progress" elseif __scrutinee.tag == "Telemetry" then return "Cooling mode restores energy and lowers heat" else error("non-exhaustive match") end end)(scene)
end

session_label = function(scene_changes)
    return (function() if (scene_changes < 3) then return "Session: warmup" elseif (scene_changes < 8) then return "Session: active" else return "Session: deep run" end end)()
end

engagement_label = function(pilot_frames, telemetry_frames)
    return (function() if (pilot_frames > telemetry_frames) then return "Engagement: pilot-heavy" elseif (telemetry_frames > pilot_frames) then return "Engagement: telemetry-heavy" else return "Engagement: balanced" end end)()
end

energy_delta = function(scene, direction)
    return (function() if is_pilot(scene) then return (function() if is_moving(direction) then return (0 - 2) else return (0 - 1) end end)() elseif is_telemetry(scene) then return 2 else return 1 end end)()
end

heat_delta = function(scene, direction)
    return (function() if is_pilot(scene) then return (function(__scrutinee) if __scrutinee.tag == "Clockwise" then return 2 elseif __scrutinee.tag == "CounterClockwise" then return 3 elseif __scrutinee.tag == "Still" then return 1 else error("non-exhaustive match") end end)(direction) elseif is_telemetry(scene) then return (0 - 3) else return (0 - 1) end end)()
end

next_combo = function(scene, direction, combo)
    return (function() if (is_pilot(scene) and is_moving(direction)) then return clamp((combo + 1), 0, 9) else return 0 end end)()
end

score_delta = function(scene, direction, combo)
    return (function() if (is_pilot(scene) and is_moving(direction)) then return (1 + combo) else return 0 end end)()
end

lap_delta = function(scene, direction, ticks)
    return (function() if ((is_pilot(scene) and is_moving(direction)) and ((ticks % 180) == 0)) then return 1 else return 0 end end)()
end

alert_from_heat = function(heat)
    return (function() if (heat >= 75) then return { tag = "Overdrive" } elseif (heat >= 40) then return { tag = "Heating" } else return { tag = "Nominal" } end end)()
end

alert_label = function(alert)
    return (function(__scrutinee) if __scrutinee.tag == "Nominal" then return "Alert: nominal" elseif __scrutinee.tag == "Heating" then return "Alert: heating" elseif __scrutinee.tag == "Overdrive" then return "Alert: overdrive" else error("non-exhaustive match") end end)(alert)
end

energy_label = function(energy)
    return (function() if (energy > 75) then return "Energy: high" elseif (energy > 40) then return "Energy: stable" elseif (energy > 15) then return "Energy: low" else return "Energy: critical" end end)()
end

heat_label = function(heat)
    return (function() if (heat < 30) then return "Heat: cool" elseif (heat < 70) then return "Heat: elevated" else return "Heat: dangerous" end end)()
end

score_label = function(score)
    return (function() if (score < 40) then return "Score tier: rookie" elseif (score < 120) then return "Score tier: pilot" else return "Score tier: ace" end end)()
end

combo_label = function(combo)
    return (function() if (combo == 0) then return "Combo: none" elseif (combo < 4) then return "Combo: building" elseif (combo < 8) then return "Combo: hot" else return "Combo: maxed" end end)()
end

lap_label = function(laps)
    return (function() if (laps == 0) then return "Laps: warming up" elseif (laps < 4) then return "Laps: patrol" else return "Laps: endurance" end end)()
end

progress_width = function(scene_changes)
    local l46 = 180
    local l47 = (scene_changes * 12)
    return (function() if (l47 > l46) then return l46 else return l47 end end)()
end

direction_pointer_y = function(direction)
    return (function(__scrutinee) if __scrutinee.tag == "Clockwise" then return 28 elseif __scrutinee.tag == "CounterClockwise" then return 52 elseif __scrutinee.tag == "Still" then return 40 else error("non-exhaustive match") end end)(direction)
end

pulse_line_y = function(ticks)
    return (function() if ((ticks % 120) < 60) then return 232 else return 228 end end)()
end

build_view = function(base)
    local _ = playdate.timer.updateTimers()
    local l51 = read_transition()
    local l52 = apply_transition(base.scene, l51)
    local l53 = read_direction(playdate.getCrankChange())
    local l54 = (base.ticks + 1)
    local l55 = next_combo(l52, l53, base.combo)
    local l56 = clamp((base.energy + energy_delta(l52, l53)), 0, 100)
    local l57 = clamp((base.heat + heat_delta(l52, l53)), 0, 100)
    local l58 = (base.score + score_delta(l52, l53, l55))
    local l59 = (base.laps + lap_delta(l52, l53, l54))
    local l60 = (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.scene = l52; __tmp.direction = l53; __tmp.side = read_side(playdate.getCrankPosition()); __tmp.ticks = l54; __tmp.scene_changes = (base.scene_changes + transition_delta(l51)); __tmp.last_transition = l51; __tmp.combo = l55; __tmp.energy = l56; __tmp.heat = l57; __tmp.score = l58; __tmp.laps = l59; __tmp.alert = alert_from_heat(l57); return __tmp end)(base)
    local l61 = (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.pilot_frames = bump_pilot_frames(l52, l60.pilot_frames); return __tmp end)(l60)
    return (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.telemetry_frames = bump_telemetry_frames(l52, l61.telemetry_frames); return __tmp end)(l61)
end

init = function()
    return { scene = { tag = "Splash" }, direction = { tag = "Still" }, side = { tag = "Left" }, alert = { tag = "Nominal" }, ticks = 0, scene_changes = 0, pilot_frames = 0, telemetry_frames = 0, score = 0, combo = 0, laps = 0, energy = 90, heat = 10, last_transition = { tag = "Stayed" } }
end

update = function(state)
    return build_view(state)
end

render = function(model)
    local _ = playdate.graphics.clear()
    local _ = playdate.graphics.drawLine(20, 74, 380, 74)
    local _ = playdate.graphics.drawLine(360, 40, 392, direction_pointer_y(model.direction))
    local l64 = pulse_line_y(model.ticks)
    local _ = playdate.graphics.drawLine(20, l64, (20 + progress_width(model.scene_changes)), l64)
    local _ = playdate.graphics.drawRect(20, 190, 104, 12)
    local _ = playdate.graphics.fillRect(22, 192, model.energy, 8)
    local _ = playdate.graphics.drawRect(140, 190, 104, 12)
    local _ = playdate.graphics.fillRect(142, 192, model.heat, 8)
    local _ = playdate.graphics.drawText("Callisto + Playdate", 20, 20)
    local _ = playdate.graphics.drawText("Auto bootstrap mission loop", 20, 40)
    local _ = playdate.graphics.drawText(ViewModel_heading(model), 20, 60)
    local _ = playdate.graphics.drawText(scene_title(model.scene), 20, 80)
    local _ = playdate.graphics.drawText(transition_label(model.last_transition), 20, 100)
    local _ = playdate.graphics.drawText(direction_label(model.direction), 20, 120)
    local _ = playdate.graphics.drawText(side_label(model.side), 20, 140)
    local _ = playdate.graphics.drawText(scene_hint(model.scene), 20, 160)
    local _ = playdate.graphics.drawText(session_label(model.scene_changes), 20, 174)
    local _ = playdate.graphics.drawText(energy_label(model.energy), 20, 206)
    local _ = playdate.graphics.drawText(heat_label(model.heat), 140, 206)
    local _ = playdate.graphics.drawText(alert_label(model.alert), 20, 220)
    local _ = playdate.graphics.drawText(score_label(model.score), 160, 220)
    local _ = playdate.graphics.drawText(combo_label(model.combo), 20, 234)
    local _ = playdate.graphics.drawText(lap_label(model.laps), 160, 234)
    local _ = playdate.graphics.drawText(engagement_label(model.pilot_frames, model.telemetry_frames), 20, 248)
    return nil
end

ViewModel_heading = function(self)
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return "A/B press to navigate scenes" elseif __scrutinee.tag == "Pilot" then return "Pilot mission: keep combo while managing heat" elseif __scrutinee.tag == "Telemetry" then return "Telemetry: monitor system recovery" else error("non-exhaustive match") end end)(self.scene)
end

M.init = init
M.update = update
M.render = render

return M
