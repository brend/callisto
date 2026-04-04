local M = {}

local function safe_play_note(freq, length, waveform)
    local snd = playdate and playdate.sound
    if not snd then
        return
    end

    local note_fn = snd.playNote
    if type(note_fn) == "function" then
        note_fn(freq, length, waveform)
        return
    end
end

function M.bounce_blip()
    safe_play_note(720.0, 0.04, 1)
end

function M.reset_chime()
    safe_play_note(440.0, 0.08, 2)
end

return M
