local M = {}

local bump

bump = function(p)
    return (function(__base) local __tmp = {}; for k, v in pairs(__base) do __tmp[k] = v end; __tmp.x = (p.x + 1); return __tmp end)(p)
end


return M
