local M = {}

local pick

pick = function(m)
    return (function(__scrutinee) if __scrutinee.tag == "Some" and true then local l1 = __scrutinee._1 return l1 elseif __scrutinee.tag == "None" then return 0 else error("non-exhaustive match") end end)(m)
end


return M
