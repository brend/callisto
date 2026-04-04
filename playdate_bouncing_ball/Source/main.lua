local game = import "game"
local ballX = 0

function playdate.update()
  game.render(ballX)
  ballX = game.next_x(ballX)
end
