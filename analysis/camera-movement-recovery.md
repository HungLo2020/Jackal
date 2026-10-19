# Jeep movement and shared camera

Source: reference commit e8fc3391d5b27fcd98b82fd27984141840449708,
annotated original instructions at E674–E885, EA02–EC67 and DD11–DD52;
controlled reference runs retained in tests/fixtures/camera_motion.json.

The native CameraController reproduces the original 64..191 horizontal
single-player camera region, 96-pixel forward threshold, and 64-pixel
recent-travel allowance for reverse scrolling. At the world's horizontal
limits the jeep is bounded to screen X16..240. Boss encounters bound
screen Y48..224. Two-player scrolling uses both jeeps' extents and last
movement directions, not an average camera target. A trailing jeep can
prevent scrolling instead of being dragged forward to accommodate its partner.

During forced boss-approach scrolling the original bypasses the player input
and firing routine. It advances the camera one pixel per tick, moves jeeps
downscreen until row208, then carries them with the camera. This path does
not add to the normal 64-pixel backtracking allowance.

Collision tests the proposed center plus two direction-specific leading
probes. Although EA7B supplies offsets before position integration, its
callee E2E8 adds the velocity, so testing the old center is incorrect. The
probe pairs are N(±4,-8), NE(0,-6)/(6,0), E(8,±4), SE(6,0)/(0,6), S(±4,8),
SW(-6,0)/(0,6), W(-8,±4), NW(-6,0)/(0,-6). Both must be traversable; stage4
also permits terrain class3 and halves movement while the center is in it.

Opposed D-pad directions select the whole idle path. Releasing direction
stops rotation and resets requested firing direction to the current visible
facing. Half-turn ties follow the original directional ordering: North to
South first turns northwest, while South to North first turns southwest.

The camera test compares logical player X/Y, horizontal camera position,
level progression and reverse allowance on every one of 3,200 frames across
ten reference scenarios. It includes shore collision, bounds, opposing
co-op movement, backtracking and forced scrolling. These checks deliberately
use original logical coordinates, not hardware sprite positions: the latter
also contain visual bobbing and the original render-buffer delay.

Reproduce with tools/reference/export_movement.py --check. Normal Cargo
builds never execute that optional research exporter.
