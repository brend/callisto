local M = {}

local draw_crank_direction
local update

draw_crank_direction = function(delta)
    return (function() if (delta > 0.0) then return playdate.graphics.drawText("Crank: +", 20, 72) elseif (delta < 0.0) then return playdate.graphics.drawText("Crank: -", 20, 72) else return playdate.graphics.drawText("Crank: 0", 20, 72) end end)()
end

update = function()
    playdate.graphics.clear()
    playdate.graphics.drawText("Callisto + Playdate", 20, 30)
    playdate.graphics.drawText("Auto bootstrap demo", 20, 50)
    draw_crank_direction(playdate.getCrankChange())
    return nil
end

M.update = update

return M
