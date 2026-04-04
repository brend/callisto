local M = {}

local spawn_ball
local unwrap_or
local clamp
local clamp_x
local clamp_y
local bounce_x
local bounce_y
local mode_label
local choose_mode
local control_delta
local sfx_code
local maybe_sfx_code
local detect_bounce_sfx
local play_sfx
local integrate_ball
local decide_step
local render
local step
local Ball_moved

spawn_ball = function()
    return { x = 200, y = 120, vx = 3, vy = 0 }
end

unwrap_or = function(value, fallback)
    return (function(__scrutinee) if __scrutinee.tag == "Some" and true then local l2 = __scrutinee._1 return l2 elseif __scrutinee.tag == "None" then return fallback else error("non-exhaustive match") end end)(value)
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

sfx_code = function(sfx)
    return (function(__scrutinee) if __scrutinee.tag == "Bounce" then return 1 else error("non-exhaustive match") end end)(sfx)
end

maybe_sfx_code = function(effect)
    return (function(__scrutinee) if __scrutinee.tag == "Some" and true then local l17 = __scrutinee._1 return { tag = "Some", _1 = sfx_code(l17) } elseif __scrutinee.tag == "None" then return { tag = "None" } else error("non-exhaustive match") end end)(effect)
end

detect_bounce_sfx = function(next_vx, prior_vx, next_vy, target_vy)
    return (function() if (next_vx ~= prior_vx) then return { tag = "Some", _1 = { tag = "Bounce" } } elseif (next_vy ~= target_vy) then return { tag = "Some", _1 = { tag = "Bounce" } } else return { tag = "None" } end end)()
end

play_sfx = function(effect)
    local l23 = unwrap_or(maybe_sfx_code(effect), 0)
    return (function() if (l23 == 1) then return playdate.audio.bounce_blip() else return nil end end)()
end

integrate_ball = function(ball, vy_delta)
    local l26 = 8
    local l27 = clamp((ball.vy + vy_delta), (0 - l26), l26)
    local l28 = Ball_moved(ball, ball.vx, l27)
    local l29 = clamp_x(l28.x)
    local l30 = clamp_y(l28.y)
    local l31 = bounce_x(l28.x, ball.vx)
    local l32 = bounce_y(l28.y, l27)
    local l33 = detect_bounce_sfx(l31, ball.vx, l32, l27)
    return { ball = (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.x = l29; __tmp.y = l30; __tmp.vx = l31; __tmp.vy = l32; return __tmp end)(l28), sfx = l33 }
end

decide_step = function(ball, mode, crank_delta)
    return (function() if playdate.input.b_just_pressed() then return { tag = "Reset" } else local l37 = integrate_ball(ball, control_delta(mode, crank_delta)); return { tag = "Continue", _1 = l37.ball, _2 = l37.sfx } end end)()
end

render = function(ball)
    local l39 = choose_mode()
    playdate.graphics.clear()
    playdate.graphics.drawText("O", ball.x, ball.y)
    playdate.graphics.drawText(mode_label(l39), 8, 8)
    playdate.graphics.drawText("A hold: d-pad mode", 8, 24)
    playdate.graphics.drawText("B: reset ball", 8, 40)
    return nil
end

step = function(ball, crank_delta)
    local l42 = choose_mode()
    return (function(__scrutinee) if __scrutinee.tag == "Continue" and true and true then local l43 = __scrutinee._1 local l44 = __scrutinee._2 play_sfx(l44); return l43 elseif __scrutinee.tag == "Reset" then playdate.audio.reset_chime(); return spawn_ball() else error("non-exhaustive match") end end)(decide_step(ball, l42, crank_delta))
end

Ball_moved = function(self, dx, dy)
    return (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.x = (self.x + dx); __tmp.y = (self.y + dy); return __tmp end)(self)
end

M.render = render
M.step = step

return M
