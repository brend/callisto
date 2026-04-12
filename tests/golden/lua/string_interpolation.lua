local M = {}

local banner

banner = function(name, count)
    return ("Hello " .. tostring(name) .. "! #" .. tostring((count + 1)) .. " ${literal}")
end


return M
