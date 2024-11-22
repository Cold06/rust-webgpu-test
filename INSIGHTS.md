# What is a PART?

A part is a irreducible piece of SVG. They are registered in solver data structures, and have params to be eligible for selection in the solver.

When selecteed it has a number of control parameters (maybe they keep the part rendered for some fun stuff.)

Parts are fully modular.

They can request rig sliders, or UI Sliders (required for eyes)

Some parts can be just mapped. Other parts (like heads, foot) have to have 1000s of variations since they only apply at a very specific time and space.


# TWO MODES OF OPERATION

## RIG CONTROLS OUTPUT

In this mode mode the rig VM updates the solver that generate the SVG JS render code 



## OUTPUT CONTROLS RIG

In this mode the SVG render code has its own control points, this can cause two effects:

1. The movement is satisfied by rig motion, so the rig is updated as such.

2. The movement is imposible to do because of rig constraints, so a rig delta is applied.
The rig delta, stores basically extra metadata used to draw the current part in such state.



