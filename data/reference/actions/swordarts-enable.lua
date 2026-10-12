function IsEnableSwordArts()
    local style = c_Style
    local arts_id = c_SwordArtsID
    local sp_kind = env(345, HAND_LEFT)
    if env(500) == TRUE then
        return FALSE
    end
    if c_Style == HAND_RIGHT and env(225, HAND_LEFT) == WEAPON_CATEGORY_STAFF then
        return FALSE
    end
    if style ~= HAND_LEFT_BOTH and c_SwordArtsHand == 0 then
        if IsShieldArts(arts_id) == TRUE then
            return TRUE
        else
            return FALSE
        end
    elseif arts_id ~= 17 and arts_id ~= SWORDARTS_INVALID then
        return TRUE
    end
    return FALSE
end
