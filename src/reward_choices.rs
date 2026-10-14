//! Choose from the native serialized triples after excluding owned abilities.
use crate::hex_pool::{HEX_COUNT, OWNER_IDS, ORDERS};

pub fn terrain_exclusions(owned: u32, great_hollow: bool) -> u32 {
    let cage=OWNER_IDS.iter().position(|&id|id==95001600).expect("Missing cage in serialized pool");
    owned | if great_hollow {1<<cage} else {0}
}

/// Native CharaInitParam / current PlayerGameData hero IDs are one-based.
/// Unknown identity must not grant a character-specific choice.
pub fn role_exclusions(unavailable: u32, hero: Option<u8>) -> u32 {
    let sword=OWNER_IDS.iter().position(|&id|id==95000900).expect("Missing Sword in serialized pool");
    let spirit=OWNER_IDS.iter().position(|&id|id==95001800).expect("Missing Spirit in serialized pool");
    let chant=OWNER_IDS.iter().position(|&id|id==95002000).expect("Missing Chant in serialized pool");
    let mighty=OWNER_IDS.iter().position(|&id|id==95001100).expect("Missing Mighty in serialized pool");
    let jiahao=OWNER_IDS.iter().position(|&id|id==95001300).expect("Missing Jiahao in serialized pool");
    let ultimate=OWNER_IDS.iter().position(|&id|id==95000400).expect("Missing Ultimate Refresh in serialized pool");
    let contract=OWNER_IDS.iter().position(|&id|id==95002600).expect("Missing Ominous Pact in serialized pool");
    let sword_gate=if matches!(hero,Some(4|6|7)) {0} else {(1<<sword)|(1<<chant)};
    let spirit_gate=if hero==Some(6) {0} else {1<<spirit};
    let mighty_gate=if matches!(hero,Some(6|7)) {1<<mighty} else {0};
    // Native hero IDs: Guardian 2, Duchess 4, Revenant 6.
    let jiahao_gate=if matches!(hero,Some(2|4|6)) {1<<jiahao} else {0};
    // Wylder, Ironeye, Raider, Recluse, Executor only; unknown roles are excluded.
    let ultimate_gate=if matches!(hero,Some(1|3|5|7|8)) {0} else {1<<ultimate};
    // Revenant 6, Recluse 7, Executor 8.
    let contract_gate=if matches!(hero,Some(6|7|8)) {1<<contract} else {0};
    unavailable | sword_gate | spirit_gate | mighty_gate | jiahao_gate | ultimate_gate | contract_gate
}

pub fn eligible(order: [usize; 3], owned: u32) -> bool {
    order.iter().all(|&i| owned & (1 << i) == 0)
}
pub fn candidates(owned: u32) -> Vec<usize> {
    ORDERS.iter().enumerate().filter_map(|(i,&o)| eligible(o,owned).then_some(i)).collect()
}
pub fn preferred(order: [usize;3], owned: u32) -> Option<usize> {
    let mut selected=Vec::with_capacity(3);
    for i in order.into_iter().chain(0..HEX_COUNT) {
        if owned & (1<<i)==0 && !selected.contains(&i) {selected.push(i);}
        if selected.len()==3 {break;}
    }
    if selected.len()!=3 {return None;}
    ORDERS.iter().position(|o|o.as_slice()==selected.as_slice())
}
/// A custom latent-power node contains one goods item in the native category.
pub fn is_hex_good(encoded: u32) -> bool {
    encoded & 0xf0000000 == 0x40000000 && OWNER_IDS.contains(&((encoded & 0x0fffffff) as i32))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn retired_ability_is_absent_and_remaining_lot_ids_are_stable(){
        use crate::hex_pool::{LOT_IDS,LEGACY_LOT_END};
        assert_eq!(HEX_COUNT,25);assert!(!OWNER_IDS.contains(&95001900));
        assert_eq!(ORDERS.len(),13800);assert_eq!(LOT_IDS.len(),ORDERS.len());
        assert_eq!(LOT_IDS.iter().copied().collect::<std::collections::HashSet<_>>().len(),LOT_IDS.len());
        for (i,&lot) in LOT_IDS.iter().enumerate(){assert!((95200000..LEGACY_LOT_END).contains(&lot));assert_eq!((lot-95200000)%10,0);if i<4080{assert_eq!(lot,95200000+i as i32*10);}if (4896..5814).contains(&i){assert!(lot>=95258140);assert!(ORDERS[i].contains(&18));}if (5814..6840).contains(&i){assert!(lot>=95268400);assert!(ORDERS[i].contains(&19));}if (6840..7980).contains(&i){assert!(lot>=95279800);assert!(ORDERS[i].contains(&20));}if (7980..9240).contains(&i){assert!(lot>=95292400);assert!(ORDERS[i].contains(&21));}if (9240..10626).contains(&i){assert!(lot>=95306260);assert!(ORDERS[i].contains(&22));}if i>=10626{assert!(lot>=95321440);assert!(ORDERS[i].contains(&23)||ORDERS[i].contains(&24));}}
        assert!(LOT_IDS[4080..].windows(2).any(|p|p[1]-p[0]>10));
    }
    #[test] fn spirit_is_revenant_only_and_bit_sixteen_is_preserved() {
        let spirit=OWNER_IDS.iter().position(|&id|id==95001800).unwrap();assert_eq!(spirit,16);
        for hero in std::iter::once(None).chain((0..=255).map(Some)) {
            let excluded=role_exclusions(0,hero);let pool=candidates(excluded);
            assert_eq!(pool.iter().any(|&i|ORDERS[i].contains(&spirit)),hero==Some(6));
            assert_eq!(ORDERS[preferred([spirit,0,1],excluded).unwrap()].contains(&spirit),hero==Some(6));
            for i in candidates(role_exclusions(1<<spirit,hero)){assert!(!ORDERS[i].contains(&spirit));}
        }
    }
    #[test] fn mighty_excludes_revenant_and_recluse_in_random_and_preferred_rewards() {
        let mighty=OWNER_IDS.iter().position(|&id|id==95001100).unwrap();
        for hero in std::iter::once(None).chain((0..=255).map(Some)) {
            let allowed=!matches!(hero,Some(6|7));
            let excluded=role_exclusions(1<<5,hero);
            let pool=candidates(excluded);
            assert_eq!(pool.iter().any(|&i|ORDERS[i].contains(&mighty)),allowed);
            let choice=ORDERS[preferred([mighty,5,0],excluded).unwrap()];
            assert_eq!(choice.contains(&mighty),allowed);
            assert!(!choice.contains(&5));
            for i in candidates(role_exclusions(1<<mighty,hero)) {
                assert!(!ORDERS[i].contains(&mighty));
            }
        }
    }
    #[test] fn ominous_pact_role_filter_applies_to_random_and_focused_rewards() {
        let contract=OWNER_IDS.iter().position(|&id|id==95002600).unwrap();
        let heart=OWNER_IDS.iter().position(|&id|id==95002700).unwrap();
        for hero in std::iter::once(None).chain((0..=255).map(Some)) {
            let allowed=!matches!(hero,Some(6|7|8));
            let excluded=role_exclusions(1<<5,hero);
            let pool=candidates(excluded);
            assert_eq!(pool.iter().any(|&i|ORDERS[i].contains(&contract)),allowed);
            for &i in &pool {assert!(eligible(ORDERS[i],excluded));}
            let focused=ORDERS[preferred([contract,heart,21],excluded).unwrap()];
            assert_eq!(focused.contains(&contract),allowed);
            assert!(focused.contains(&heart));assert!(!focused.contains(&5));
            assert!(candidates(role_exclusions(1<<contract,hero)).iter().all(|&i|!ORDERS[i].contains(&contract)));
        }
    }
    #[test] fn sword_only_appears_for_duchess_revenant_and_recluse_in_every_reward_entry() {
        let sword=OWNER_IDS.iter().position(|&id|id==95000900).unwrap();
        let chant=OWNER_IDS.iter().position(|&id|id==95002000).unwrap();
        let cage=OWNER_IDS.iter().position(|&id|id==95001600).unwrap();
        for hero in std::iter::once(None).chain((0..=255).map(Some)) {
            let allowed=matches!(hero,Some(4|6|7));
            for hollow in [false,true] {
                let excluded=role_exclusions(terrain_exclusions(1<<5,hollow),hero);
                let pool=candidates(excluded);
                assert_eq!(pool.iter().any(|&i|ORDERS[i].contains(&chant)),allowed);
                assert_eq!(pool.iter().any(|&i|ORDERS[i].contains(&sword)),allowed);
                for i in pool {
                    assert!(!ORDERS[i].contains(&5));
                    if !allowed {assert!(!ORDERS[i].contains(&sword));}
                    if hollow {assert!(!ORDERS[i].contains(&cage));}
                }
                let selected=ORDERS[preferred([sword,5,6],excluded).unwrap()];
                assert_eq!(selected.contains(&sword),allowed);
                assert!(!selected.contains(&5));
                // Already-owned Sword stays excluded even on an eligible hero.
                assert_ne!(role_exclusions(excluded|1<<sword,hero)&(1<<sword),0);
            }
        }
    }
    #[test] fn great_hollow_excludes_cage_from_both_random_and_preferred_choices() {
        let cage=OWNER_IDS.iter().position(|&id|id==95001600).unwrap();
        let unavailable=terrain_exclusions(1<<3,true);
        for i in candidates(unavailable) {assert!(!ORDERS[i].contains(&cage));assert!(!ORDERS[i].contains(&3));}
        let choice=ORDERS[preferred([cage,3,6],unavailable).unwrap()];
        assert!(!choice.contains(&cage));assert!(!choice.contains(&3));
        assert_eq!(terrain_exclusions(1<<3,false),1<<3);
        assert!(ORDERS[preferred([cage,3,6],0).unwrap()].contains(&cage));
    }
    #[test] fn representative_ownership_combinations_exclude_owned_choices() {
        // Exhaustive selection is cubic in pool size; sample masks across every
        // bit and small remaining pools, including the newest abilities.
        let all=(1u32<<HEX_COUNT)-1;
        let masks=(0..256).map(|i|(i as u32).wrapping_mul(0x1f123bb5) & all)
            .chain([0,all]).chain((0..HEX_COUNT).flat_map(|i|[1<<i,all^(1<<i)]))
            .chain(ORDERS.iter().map(|o|all^((1<<o[0])|(1<<o[1])|(1<<o[2]))));
        for owned in masks {
            let available=HEX_COUNT-owned.count_ones() as usize;
            let expected=if available>=3 {available*(available-1)*(available-2)} else {0};
            let pool=candidates(owned);assert_eq!(pool.len(),expected);
            for i in pool {assert!(eligible(ORDERS[i],owned));}
        }
    }
    #[test] fn test_hotkeys_also_avoid_duplicates_and_keep_available_preferences() {
        assert_eq!(ORDERS[preferred([9,3,6],1<<3).unwrap()],[9,6,0]);
        assert_eq!(preferred([9,3,6],(1<<HEX_COUNT)-1),None);
        assert!(is_hex_good(0x40000000|95001100));
        assert!(!is_hex_good(95001100));assert!(!is_hex_good(0x40000000|95001000));
    }
}
