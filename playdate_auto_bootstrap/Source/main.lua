local game = import "game"
local __state = game.init()

function playdate.update()
    __state = game.update(__state)
    game.render(__state)
end
