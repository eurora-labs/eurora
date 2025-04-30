struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@group(0) @binding(0)
var r_texture: texture_2d<f32>;
@group(0) @binding(1)
var r_sampler: sampler;



struct UniformData {
    opacity: f32,
     texel_size_x: f32,
    texel_size_y: f32,
    blur: f32,
} 


@group(0) @binding(2)
var<uniform> uniforms: UniformData;


@vertex
fn vs_main(@location(0) pos: vec2f, @location(1) uv: vec2f) -> VertexOutput {
    var out: VertexOutput;
    
    out.pos = vec4f(pos, 0.0, 1.0);
    out.uv = uv;
    out.uv.y = 1.-out.uv.y ;
    return out;
}




const SAMPLES: i32 = 35;



@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
   
    let texel_size = vec2f(uniforms.texel_size_x, uniforms.texel_size_y); 

      let weights = array<array<f32, 3>, 3>(
        array<f32, 3>(1.0, 2.0, 1.0),
        array<f32, 3>(2.0, 4.0, 2.0),
        array<f32, 3>(1.0, 2.0, 1.0),
    );

    var color = vec4f(0.0);
    var total = 0.0;




    for (var dx = -SAMPLES / 2; dx <= SAMPLES / 2; dx++) {
        for (var dy = -SAMPLES / 2; dy <= SAMPLES / 2; dy++) {
            let offset = vec2f(f32(dx), f32(dy)) * texel_size;
            let weight = weights[(dx + 1)][(dy + 1)];
            color += textureSample(r_texture, r_sampler,in.uv + offset) * weight;
            total += weight;
        }
    }

  
  let  blurred =vec4f(color.rgb / total,1.0);   
//   let tex = textureSample(r_texture, r_sampler, in.uv); 
//   let final_blur = mix (tex,blurred, uniforms.blur);

   
var gray = vec4(169./255.,169./255.,169./255.,1.0 );
gray.r*= uniforms.opacity;;
gray.g*= uniforms.opacity;;
gray.b*= uniforms.opacity;;
    return mix(blurred, gray, uniforms.opacity);
}