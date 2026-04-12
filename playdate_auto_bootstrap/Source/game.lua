local M = {}

local k_button_b
local k_button_a
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
local pulse_label
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
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return "Press A: next scene, B: previous scene" elseif __scrutinee.tag == "Pilot" then return "Scene persists until next A/B press" elseif __scrutinee.tag == "Telemetry" then return "Tracks pilot/telemetry engagement over time" else error("non-exhaustive match") end end)(scene)
end

session_label = function(scene_changes)
    return (function() if (scene_changes < 3) then return "Session: warmup" elseif (scene_changes < 8) then return "Session: active" else return "Session: deep run" end end)()
end

engagement_label = function(pilot_frames, telemetry_frames)
    return (function() if (pilot_frames > telemetry_frames) then return "Engagement: pilot-heavy" elseif (telemetry_frames > pilot_frames) then return "Engagement: telemetry-heavy" else return "Engagement: balanced" end end)()
end

pulse_label = function(ticks)
    return (function() if ((ticks % 120) < 60) then return "Pulse: high" else return "Pulse: low" end end)()
end

build_view = function(base)
    local l21 = read_transition()
    local l22 = apply_transition(base.scene, l21)
    local l23 = (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.scene = l22; __tmp.direction = read_direction(playdate.getCrankChange()); __tmp.side = read_side(playdate.getCrankPosition()); __tmp.ticks = (base.ticks + 1); __tmp.scene_changes = (base.scene_changes + transition_delta(l21)); __tmp.last_transition = l21; return __tmp end)(base)
    local l24 = (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.pilot_frames = bump_pilot_frames(l22, l23.pilot_frames); return __tmp end)(l23)
    return (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.telemetry_frames = bump_telemetry_frames(l22, l24.telemetry_frames); return __tmp end)(l24)
end

init = function()
    return { scene = { tag = "Splash" }, direction = { tag = "Still" }, side = { tag = "Left" }, ticks = 0, scene_changes = 0, pilot_frames = 0, telemetry_frames = 0, last_transition = { tag = "Stayed" } }
end

update = function(state)
    return build_view(state)
end

render = function(model)
    local _ = playdate.graphics.clear()
    local _ = playdate.graphics.drawText("Callisto + Playdate", 20, 20)
    local _ = playdate.graphics.drawText("Auto bootstrap state demo", 20, 40)
    local _ = playdate.graphics.drawText(ViewModel_heading(model), 20, 60)
    local _ = playdate.graphics.drawText(scene_title(model.scene), 20, 80)
    local _ = playdate.graphics.drawText(transition_label(model.last_transition), 20, 100)
    local _ = playdate.graphics.drawText(direction_label(model.direction), 20, 120)
    local _ = playdate.graphics.drawText(side_label(model.side), 20, 140)
    local _ = playdate.graphics.drawText(scene_hint(model.scene), 20, 160)
    local _ = playdate.graphics.drawText(session_label(model.scene_changes), 20, 180)
    local _ = playdate.graphics.drawText(engagement_label(model.pilot_frames, model.telemetry_frames), 20, 200)
    local _ = playdate.graphics.drawText(pulse_label(model.ticks), 20, 220)
    return nil
end

ViewModel_heading = function(self)
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return "A/B press to navigate scenes" elseif __scrutinee.tag == "Pilot" then return "Pilot: crank to steer" elseif __scrutinee.tag == "Telemetry" then return "Telemetry: inspect session stats" else error("non-exhaustive match") end end)(self.scene)
end

M.init = init
M.update = update
M.render = render

return M
