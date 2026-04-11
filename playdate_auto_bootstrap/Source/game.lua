local M = {}

local k_button_b
local k_button_a
local read_scene
local read_direction
local read_side
local scene_title
local direction_label
local side_label
local scene_hint
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

read_scene = function()
    return (function() if playdate.buttonIsPressed(k_button_a()) then return { tag = "Pilot" } elseif playdate.buttonIsPressed(k_button_b()) then return { tag = "Telemetry" } else return { tag = "Splash" } end end)()
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

scene_hint = function(scene)
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return "Release A/B to return here" elseif __scrutinee.tag == "Pilot" then return "Use crank for direction changes" elseif __scrutinee.tag == "Telemetry" then return "Watch side + direction labels" else error("non-exhaustive match") end end)(scene)
end

build_view = function(base)
    local l7 = (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.scene = read_scene(); return __tmp end)(base)
    local l8 = (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.direction = read_direction(playdate.getCrankChange()); return __tmp end)(l7)
    return (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.side = read_side(playdate.getCrankPosition()); return __tmp end)(l8)
end

init = function()
    return { scene = { tag = "Splash" }, direction = { tag = "Still" }, side = { tag = "Left" } }
end

update = function(state)
    return build_view(state)
end

render = function(model)
    playdate.graphics.clear()
    playdate.graphics.drawText("Callisto + Playdate", 20, 30)
    playdate.graphics.drawText("Auto bootstrap state demo", 20, 50)
    playdate.graphics.drawText(ViewModel_heading(model), 20, 70)
    playdate.graphics.drawText(scene_title(model.scene), 20, 90)
    playdate.graphics.drawText(direction_label(model.direction), 20, 110)
    playdate.graphics.drawText(side_label(model.side), 20, 130)
    playdate.graphics.drawText(scene_hint(model.scene), 20, 150)
    return nil
end

ViewModel_heading = function(self)
    return (function(__scrutinee) if __scrutinee.tag == "Splash" then return "Hold A/B to switch scene" elseif __scrutinee.tag == "Pilot" then return "Pilot: crank to steer" elseif __scrutinee.tag == "Telemetry" then return "Telemetry: inspect input state" else error("non-exhaustive match") end end)(self.scene)
end

M.init = init
M.update = update
M.render = render

return M
