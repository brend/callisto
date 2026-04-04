local M = {}

local spawn_ball
local clamp
local clamp_x
local clamp_y
local bounce_x
local bounce_y
local mode_label
local choose_mode
local control_delta
local integrate_ball
local decide_step
local render
local step
local Ball_moved

spawn_ball = function()
    return { x = 200, y = 120, vx = 3, vy = 0 }
end

clamp = function(v, lo, hi)
    return (function() if (v < lo) then return lo elseif (v > hi) then return hi else return v end end)()
end

clamp_x = function(v)
    return clamp(v, 0, 380)
end

clamp_y = function(v)
    return clamp(v, 20, 220)
end

bounce_x = function(moved_x, vx)
    return (function() if (moved_x < 0) then return (0 - vx) elseif (moved_x > 380) then return (0 - vx) else return vx end end)()
end

bounce_y = function(moved_y, vy)
    return (function() if (moved_y < 20) then return (0 - vy) elseif (moved_y > 220) then return (0 - vy) else return vy end end)()
end

mode_label = function(mode)
    return (function(__scrutinee) if __scrutinee.tag == "Crank" then return "Mode: Crank (hold A for d-pad)" elseif __scrutinee.tag == "Buttons" then return "Mode: D-pad (B resets)" else error("non-exhaustive match") end end)(mode)
end

choose_mode = function()
    return (function() if playdate.input.a_pressed() then return { tag = "Buttons" } else return { tag = "Crank" } end end)()
end

control_delta = function(mode, crank_delta)
    return (function(__scrutinee) if __scrutinee.tag == "Crank" then return (crank_delta / 4) elseif __scrutinee.tag == "Buttons" then return (function() if playdate.input.up_pressed() then return (0 - 2) elseif playdate.input.down_pressed() then return 2 else return 0 end end)() else error("non-exhaustive match") end end)(mode)
end

integrate_ball = function(ball, vy_delta)
    local l14 = 8
    local l15 = clamp((ball.vy + vy_delta), (0 - l14), l14)
    local l16 = Ball_moved(ball, ball.vx, l15)
    local l17 = clamp_x(l16.x)
    local l18 = clamp_y(l16.y)
    local l19 = bounce_x(l16.x, ball.vx)
    local l20 = bounce_y(l16.y, l15)
    return (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.x = l17; __tmp.y = l18; __tmp.vx = l19; __tmp.vy = l20; return __tmp end)(l16)
end

decide_step = function(ball, mode, crank_delta)
    return (function() if playdate.input.b_just_pressed() then return { tag = "Reset" } else return { tag = "Continue", _1 = integrate_ball(ball, control_delta(mode, crank_delta)) } end end)()
end

render = function(ball)
    local l25 = choose_mode()
    playdate.graphics.clear()
    playdate.graphics.drawText("O", ball.x, ball.y)
    playdate.graphics.drawText(mode_label(l25), 8, 8)
    playdate.graphics.drawText("A hold: d-pad mode", 8, 24)
    playdate.graphics.drawText("B: reset ball", 8, 40)
    return nil
end

step = function(ball, crank_delta)
    local l28 = choose_mode()
    return (function(__scrutinee) if __scrutinee.tag == "Continue" and true then local l29 = __scrutinee._1 return l29 elseif __scrutinee.tag == "Reset" then return spawn_ball() else error("non-exhaustive match") end end)(decide_step(ball, l28, crank_delta))
end

Ball_moved = function(self, dx, dy)
    return (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.x = (self.x + dx); __tmp.y = (self.y + dy); return __tmp end)(self)
end

M.render = render
M.step = step

return M
