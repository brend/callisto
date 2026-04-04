local M = {}

local k_button_left
local k_button_right
local k_button_up
local k_button_down
local k_button_b
local k_button_a
local left_pressed
local right_pressed
local up_pressed
local down_pressed
local a_pressed
local b_just_pressed

k_button_left = function()
    return 1
end

k_button_right = function()
    return 2
end

k_button_up = function()
    return 4
end

k_button_down = function()
    return 8
end

k_button_b = function()
    return 16
end

k_button_a = function()
    return 32
end

left_pressed = function()
    return playdate.buttonIsPressed(k_button_left())
end

right_pressed = function()
    return playdate.buttonIsPressed(k_button_right())
end

up_pressed = function()
    return playdate.buttonIsPressed(k_button_up())
end

down_pressed = function()
    return playdate.buttonIsPressed(k_button_down())
end

a_pressed = function()
    return playdate.buttonIsPressed(k_button_a())
end

b_just_pressed = function()
    return playdate.buttonJustPressed(k_button_b())
end

M.left_pressed = left_pressed
M.right_pressed = right_pressed
M.up_pressed = up_pressed
M.down_pressed = down_pressed
M.a_pressed = a_pressed
M.b_just_pressed = b_just_pressed

return M
