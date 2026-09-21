## Notes on angle repose

Ball radius is max r_ball = 0.005 with a gaussian random amount subtracted specified by std dr_ball=0.25. Some green particles positioned on base of box are immovable.

step 1:
e=0.1
Y=1e-6
dt=5e-6
remove charge interactions if you want to speed up further.

wait for particles to settle

step 2:
e=0.7
Y=5e-6
dt=1e-6

wait to settle

step 3:
start gently tilting surface and wait for sustained movement = 1 particle changing sides few s of viewing time.

[1,1,0.015], [1,3,0.015], [3,1,0.015], [3,3,0.015]],

No charges --> 11 degrees
Central Charge 3e-9 --> 17/18 degrees
Dipole Charge 3e-9, d_r=0.6 --> 17 degrees.

Increase modulus to 5e7.