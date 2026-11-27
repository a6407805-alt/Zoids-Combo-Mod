Test 2: Story Mode character portraits, facing direction, and pilot nameplates fixed for Battle Legends scenes.

- Appends all 75 Battle Legends story portrait sprites (`menu/demo/demosell00.dat` -> Vs. III `ModelSet`s `98..172`), flips horizontal UVs to match Vs. III's left/right portrait mirroring, and maps `demoPicMan` + `etc/n_Person.dat` pilot nameplates (`SHOMA`, `KOUKI`, `TITA`, `ZAN`, `REINER`, `REPUBLIC OP`, `EMPIRE OP`, etc.).
- Ports 5 Battle Legends scenes + missions into Vs. III's linear opening slots `002..006` so major Republic, Empire, and Terra Geist characters can be verified back-to-back from New Game ("Don't take tutorial"):
  1. Slot `002` <- BL `000` ("Who is it?"): Republic Operator, Shoma, Kouki
  2. Slot `003` <- BL `008` ("Earth is Burning"): Zan, Tita, Shoma, Republic Operator
  3. Slot `004` <- BL `021` ("Hit it on the Head"): Reiner, Kouki, Empire Operator
  4. Slot `005` <- BL `009` ("Giant Shadow"): Reika, Leviathe, Empire Soldier, Zan, Tita, Shoma
  5. Slot `006` <- BL `012` ("Suspicion of the Crimson"): Albane, Tita, Shoma
