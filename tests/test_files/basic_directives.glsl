// type, range and default hints for various uniforms and push constants
#pragma float super.name : range(0..1) = 0.5
#pragma color var = [1.0, 1.0, 0.0, 1.0]
#pragma bool grand_parent.parent.bool_name = true
#pragma bool bool_2_name = false
#pragma vec2 point : range([-1.0, -1.0]..[1.0, 1.0]) = [0.0, 0.0]

#pragma label "Base Label"

// sampler binding type is inferred from filter
// nearest = 0 / Nonfiltering
#pragma sampler samp (filter=Linear, wrap=ClampToEdge)

// a comparison sampler
#pragma sampler dumb (comparison=NotEqual, wrap=ClampToEdge)

// dynamic offset will force you to provide dynamic offset
// during a render call
#pragma uniform uni_name (dynamic_offset=true, calculate_min_size=true)

#pragma texture multisampled (sample_count=2)

// global options
#pragma default uniform (calculate_size=true)
#pragma default sampler (filter=Linear)
