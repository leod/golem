use blinds::traits::*;
use blinds::*;
use golem::{Attribute, AttributeType, ColorFormat, Context, GeometryMode, GolemError, Dimension::D2, ElementBuffer, Texture, Uniform, UniformType, UniformValue, VertexBuffer, ShaderProgram, ShaderDescription};

async fn app(window: Window, ctx: glow::Context, mut events: EventStream) -> Result<(), GolemError> {
    let ctx = &Context::from_glow(ctx)?;

    let image = [
        // R, G, B
        255, 255, 255,
        0, 255, 0,
        255, 0, 0,
        255, 255, 255,
        0, 0, 255
    ];

    let mut texture = Texture::new(&ctx)?;
    texture.set_image(Some(&image), 2, 2, ColorFormat::RGB);

    let vertices = [
        // Position         UV
        -0.5, -0.5,         0.0, 0.0,
        0.5, -0.5,          1.0, 0.0,
        0.5, 0.5,           1.0, 1.0,
        -0.5, 0.5,          0.0, 1.0,
    ];
    let indices = [
        0, 1, 2,
        2, 3, 0,
    ];

    let mut shader = ShaderProgram::new(ctx, ShaderDescription {
        vertex_input: &[
            Attribute::new("vert_position", AttributeType::Vector(D2)),
            Attribute::new("vert_uv", AttributeType::Vector(D2)),
        ],
        fragment_input: &[
            Attribute::new("frag_uv", AttributeType::Vector(D2)),
        ],
        uniforms: &[ Uniform::new("image", UniformType::Sampler2D) ],
        vertex_shader: r#" void main() {
            gl_Position = vec4(vert_position, 0, 1);
            frag_uv = vert_uv;
        }"#,
        fragment_shader:
        r#" void main() {
            gl_FragColor = texture(image, frag_uv);
        }"#
    })?;

    let mut vb = VertexBuffer::new(ctx)?;
    let mut eb = ElementBuffer::new(ctx)?;
    vb.set_data(&vertices);
    eb.set_data(&indices);
    shader.bind(&vb);
    shader.set_uniform("image", UniformValue::Int(0))?;

    Texture::bind(ctx, Some(&texture), 0);

    ctx.clear();
    shader.draw(&eb, 0..indices.len(), GeometryMode::Triangles)?;
    window.present();

    while let Some(_) = events.next().await {
    }

    Ok(())
}

fn main() {
    run_gl(Settings::default(), |window, gfx, events| async move {
        app(window, gfx, events).await.unwrap()
    });
}
