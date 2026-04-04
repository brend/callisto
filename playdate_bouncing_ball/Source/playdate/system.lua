local M = {}

local crank_is_right_half
local crank_is_top_half

crank_is_right_half = function()
    return (playdate.getCrankPosition() >= 180.0)
end

crank_is_top_half = function()
    local l0 = playdate.getCrankPosition()
    return ((l0 >= 90.0) and (l0 <= 270.0))
end

M.crank_is_right_half = crank_is_right_half
M.crank_is_top_half = crank_is_top_half

return M
