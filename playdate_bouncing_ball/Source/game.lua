local M = {}

local clamp
local clamp_x
local clamp_y
local bounce_x
local bounce_y
local render
local step

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

render = function(ball)
    playdate.graphics.clear()
    playdate.graphics.drawText("O", ball.x, ball.y)
    playdate.graphics.drawText("Crank to curve", 8, 8)
    return nil
end

step = function(ball, crank_delta)
    local l12 = 8
    local l13 = (ball.vy + (crank_delta / 4))
    local l14 = clamp(l13, (0 - l12), l12)
    local l15 = (ball.x + ball.vx)
    local l16 = (ball.y + l14)
    local l17 = clamp_x(l15)
    local l18 = clamp_y(l16)
    local l19 = bounce_x(l15, ball.vx)
    local l20 = bounce_y(l16, l14)
    return { x = l17, y = l18, vx = l19, vy = l20 }
end

M.render = render
M.step = step

return M
