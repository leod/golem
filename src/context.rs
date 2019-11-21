use glow::HasContext;
use crate::GolemError;
use crate::buffer::{Buffer, ElementBuffer, VertexBuffer};
use crate::objects::{ColorFormat, DrawList, Surface, Texture, UniformValue};
use crate::program::{Attribute, Position, Uniform, ShaderDescription, ShaderProgram};
use std::rc::Rc;

pub struct Context {
    gl: Rc<glow::Context>,
    vao: u32 // TODO: manage this
}

fn generate_shader_text(body: &str, inputs: &[Attribute], outputs: &[Attribute], uniforms: &[Uniform]) -> String {
    let mut shader = String::new();

    #[cfg(not(target_arch = "wasm32"))]
    shader.push_str("#version 150\n");
    
    shader.push_str("precision mediump float;\n");
    for attr in inputs.iter() {
        attr.as_glsl(Position::Input, &mut shader);
    }
    for attr in outputs.iter() {
        attr.as_glsl(Position::Output, &mut shader);
    }
    for uniform in uniforms.iter() {
        uniform.as_glsl(&mut shader);
    }
    shader.push_str(body);

    shader
}

impl Context {
    pub fn from_glow(gl: glow::Context) -> Context {
        // TODO: re-set this at all?
        #[cfg(not(target_arch = "wasm32"))]
        let vao = unsafe {
            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            vao
        };
        #[cfg(target_arch = "wasm32")]
        let vao = 0;

        let gl = Rc::new(gl);


        Context {
            gl,
            vao
        }
    }

    pub fn new_shader(&self, desc: ShaderDescription) -> Result<ShaderProgram, GolemError> {
        let gl = &self.gl;
        // TODO: check for shader creation errors
        // TODO: OpenGL will drop unused variables, that's probably going to bite me?
        unsafe {
            let vertex = gl.create_shader(glow::VERTEX_SHADER).expect("TODO");
            let vertex_source = generate_shader_text(desc.vertex_shader, desc.vertex_input, desc.fragment_input, desc.uniforms);
            println!("{}", vertex_source);
            gl.shader_source(vertex, &vertex_source);
            gl.compile_shader(vertex);
            println!("{}", gl.get_shader_info_log(vertex));

            let fragment = gl.create_shader(glow::FRAGMENT_SHADER).expect("TODO");
            // Handle creating the output color and giving it a name, but only on desktop gl
            #[cfg(target_arch = "wasm32")]
            let (fragment_output, fragment_body) = {
                (&[], desc.fragment_input)
            };
            #[cfg(not(target_arch = "wasm32"))]
            let (fragment_output, fragment_body) = {
                (&[ Attribute::Vector(4, "outputColor") ], &desc.fragment_shader.replace("gl_FragColor", "outputColor"))
            };
            let fragment_source = generate_shader_text(fragment_body, desc.fragment_input, fragment_output, desc.uniforms);
            gl.shader_source(fragment, &fragment_source);
            gl.compile_shader(fragment);
            println!("{}", gl.get_shader_info_log(fragment));
            let id = gl.create_program().expect("TODO");

            gl.attach_shader(id, vertex);
            gl.attach_shader(id, fragment);

            // Bind the color output for desktop GL
            #[cfg(not(target_arch = "wasm32"))]
            gl.bind_frag_data_location(id, 0, "outputColor");

            for (index, attr) in desc.vertex_input.iter().enumerate() {
                gl.bind_attrib_location(id, index as u32, attr.name());
            }

            gl.link_program(id);

            Ok(ShaderProgram {
                id,
                vertex,
                fragment,
                input: desc.vertex_input.iter().cloned().collect(),
            })
        }
    }

    fn new_buffer(&self) -> Buffer {
        let id = unsafe { self.gl.create_buffer() }.expect("TODO");
        let ctx = Context {
            gl: self.gl.clone(),
            vao: 0,
        };

        Buffer {
            ctx,
            id,
            length: 0
        }
    }

    pub fn new_vertex_buffer(&self) -> VertexBuffer {
        VertexBuffer(self.new_buffer())
    }

    pub fn new_element_buffer(&self) -> ElementBuffer {
        ElementBuffer(self.new_buffer())
    }

    pub fn new_texture(&self, image: &[u8], width: u32, height: u32, color: ColorFormat) -> Texture {
        let format = match color {
            ColorFormat::RGB => glow::RGB,
            ColorFormat::RGBA => glow::RGBA
        };
        let gl = &self.gl;
        unsafe {
            let id = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(id));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, width as i32, 
                            height as i32, 0, format, glow::UNSIGNED_BYTE, Some(image));
            //gl.generate_mipmap(glow::TEXTURE_2D);
            gl.bind_texture(glow::TEXTURE_2D, None);

            Texture { id }
        }
    }

    pub fn bind_texture(&self, tex: &Texture, texture_unit: u32) {
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + texture_unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex.id));
        }
    }

    pub(crate) fn bind(&self, buffer: &Buffer, target: u32) {
        unsafe {
            self.gl.bind_buffer(target, Some(buffer.id));
        }
        self.errors("bind_buffer");
    }
    
    pub(crate) fn send_data<T: bytemuck::Pod>(&self, bind: u32, length: usize, start: usize, data: &[T]) {
        use std::mem::size_of;
        let data_length = size_of::<T>() * data.len();
        let data_start = size_of::<T>() * start;
        let u8_buffer = bytemuck::cast_slice(data);
        unsafe {
            if data_length + start > length {
                let new_length = data_length + data_start;
                self.gl.buffer_data_size(bind, new_length as i32 * 2, glow::STREAM_DRAW);
                self.errors("data_size");
            }
            self.gl.buffer_sub_data_u8_slice(bind, start as i32, u8_buffer);
            self.errors("u8_slice");
        };
    }

    pub fn set_target(&mut self, surface: &Surface) {
        unimplemented!();
    }

    pub fn reset_target(&mut self) {

    }

    pub fn clear(&mut self, r: f32, g: f32, b: f32, a: f32) {
        unsafe {
            self.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            self.gl.clear_color(r, g, b, a);
        }
    }

    // TODO: API allow glow::LINES
    pub fn draw(&mut self, shader: &ShaderProgram, vb: &VertexBuffer, eb: &ElementBuffer, draw_list: &[DrawList]) {
        unsafe {
            self.gl.use_program(Some(shader.id));
        }
        self.bind(&vb.0, glow::ARRAY_BUFFER);
        self.bind(&eb.0, glow::ELEMENT_ARRAY_BUFFER);
        self.errors("program and bind");
        use std::mem::size_of;
        let stride: i32 = shader.input.iter().map(|attr| attr.size()).sum();
        let stride = stride * size_of::<f32>() as i32;
        let mut offset = 0;
        for (index, attr) in shader.input.iter().enumerate() {
            let size = attr.size();
            unsafe {
                let pos_attrib = index as u32;
                self.gl.enable_vertex_attrib_array(pos_attrib);
                self.errors("enable");
                self.gl.vertex_attrib_pointer_f32(pos_attrib, size, glow::FLOAT, false, stride, offset);
                self.errors("pointer");
            }
            offset += size * size_of::<f32>() as i32;
        }
        self.errors("attributes");
        draw_list.iter().for_each(|draw_list| {
            for (name, value) in draw_list.uniforms.iter() { 
                let location = unsafe { self.gl.get_uniform_location(shader.id, name) };
                self.bind_uniform(location.unwrap(), value.clone());
            }
            let range = draw_list.range.clone();
            let length = range.end - range.start;
            unsafe {
                self.gl.draw_elements(glow::TRIANGLES, length as i32, glow::UNSIGNED_INT, range.start as i32);
            }
            self.errors("draw");
        });
    }

    fn bind_uniform(&self, location: u32, uniform: UniformValue) {
        use UniformValue::*;
        let location = Some(location);
        unsafe {
            match uniform {
                Int(x) => self.gl.uniform_1_i32(location, x),
                IVector2([x, y]) => self.gl.uniform_2_i32(location, x, y),
                IVector3([x, y, z]) => self.gl.uniform_3_i32(location, x, y, z),
                IVector4([x, y, z, w]) => self.gl.uniform_4_i32(location, x, y, z, w),
                Float(x) => self.gl.uniform_1_f32(location, x),
                Vector2([x, y]) => self.gl.uniform_2_f32(location, x, y),
                Vector3([x, y, z]) => self.gl.uniform_3_f32(location, x, y, z),
                Vector4([x, y, z, w]) => self.gl.uniform_4_f32(location, x, y, z, w),
            }
        }
    }

    fn errors(&self, label: &str) {
        let mut any = false;
        loop {
            let error = unsafe { self.gl.get_error() };
            let text = match error {
                0 => break,
                glow::INVALID_ENUM => "Invalid enum",
                glow::INVALID_VALUE => "Invalid value",
                glow::INVALID_OPERATION => "Invalid operation",
                _ => "Unknown error",
            };
            any = true;
            println!("{}", text);
        }
        if any {
            println!("{} errors complete", label);
        }
    }
}
