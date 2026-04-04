local game = import "game"
playdate.input = import "playdate/input"
local ball = { x = 200, y = 120, vx = 3, vy = 0 }

local function round_to_int(v)
    if v >= 0 then
        return math.floor(v + 0.5)
    end
    return math.ceil(v - 0.5)
end

function playdate.update()
    game.render(ball)
    local crank_delta = round_to_int(playdate.getCrankChange())
    ball = game.step(ball, crank_delta)
end
