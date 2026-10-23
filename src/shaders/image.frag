#version 440

// Draws an image like `object-fit: cover` within rounded corners, optionally with CSS's blur()
// and brightness() filters. The blur samples a mipmapped texture, which takes a handful of taps
// instead of a pass per direction.

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    // the item's size and corner radius, in device pixels
    vec2 size;
    float radius;
    // maps the item's coordinates to the texture's, cropping it to cover the item
    vec2 uvScale;
    vec2 uvOffset;
    // the blur's standard deviation in device pixels, or 0 for none
    float blur;
    // the mip level bias that the blur samples with
    float blurBias;
    float brightness;
    // 1 to fade to transparent at the edges, as a blur() filter does, or 0 to extend the edge
    // pixels, as a backdrop-filter does
    float fadeEdges;
};

layout(binding = 1) uniform sampler2D source;

// the standard normal distribution's CDF, within 0.01
float normalCdf(float x)
{
    return 1.0 / (1.0 + exp(-1.702 * x));
}

void main()
{
    vec2 uv = qt_TexCoord0 * uvScale + uvOffset;
    vec4 color;
    if (blur > 0.0) {
        // a 3x3 binomial kernel over the blurred mip level approximates a Gaussian
        vec2 spacing = 1.15 * blur / size * uvScale;
        color = vec4(0.0);
        for (int i = -1; i <= 1; ++i) {
            for (int j = -1; j <= 1; ++j) {
                float weight = (i == 0 ? 0.5 : 0.25) * (j == 0 ? 0.5 : 0.25);
                color += weight * texture(source, uv + vec2(float(i), float(j)) * spacing, blurBias);
            }
        }
    } else {
        color = texture(source, uv);
    }

    // brightness() scales unpremultiplied color and clamps it
    if (brightness != 1.0 && color.a > 0.0)
        color.rgb = min(color.rgb / color.a * brightness, 1.0) * color.a;

    vec2 p = qt_TexCoord0 * size;
    if (fadeEdges > 0.0 && blur > 0.0) {
        // blurring transparency in from beyond the edges
        color *= normalCdf(p.x / blur) * normalCdf((size.x - p.x) / blur)
            * normalCdf(p.y / blur) * normalCdf((size.y - p.y) / blur);
    }

    vec2 q = abs(p - 0.5 * size) - (0.5 * size - radius);
    float distance = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
    float coverage = clamp(0.5 - distance, 0.0, 1.0);

    fragColor = color * coverage * qt_Opacity;
}
