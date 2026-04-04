local M = {}

local wave_square
local wave_triangle
local bounce_blip
local reset_chime

wave_square = function()
    return 1
end

wave_triangle = function()
    return 2
end

bounce_blip = function()
    playdate.sound.playNote(720.0, 0.04, wave_square())
    return nil
end

reset_chime = function()
    playdate.sound.playNote(440.0, 0.08, wave_triangle())
    return nil
end

M.bounce_blip = bounce_blip
M.reset_chime = reset_chime

return M
