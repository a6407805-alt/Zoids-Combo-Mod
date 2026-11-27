Test 7: Gauntlet (same as test 6) with Antigravity's floor-clipping fix and Rayse Tiger data change turned OFF.

Why: in test 6, using an ability dragged the Zoid to the arena edge and froze it. The floor fix runs every frame and snaps any Zoid below the floor back up, marks it landed and zeroes its fall speed; abilities that move a Zoid low or rely on their own landing timing break under it.

Check: use the same ability that froze you before. If it works now, the floor fix was the cause. (The old floor-clipping glitch it fixed may come back; report where if it does.)
