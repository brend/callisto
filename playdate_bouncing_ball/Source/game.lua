local M = {}

local render
local next_x

render = function(ball_x)
    playdate.graphics.clear()
    playdate.graphics.drawText("O", ball_x, 120)
    return nil
end

next_x = function(ball_x)
    local l2 = (ball_x + 3)
    return (function() if (l2 > 380) then return 0 else return l2 end end)()
end

M.render = render
M.next_x = next_x

return M
