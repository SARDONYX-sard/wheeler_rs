//! Dear ImGui DirectX 11 renderer: a port of upstream `imgui_impl_dx11.cpp`
//! (the backend the C++ Wheeler links) on top of `libskyrim::rex` COM types.

use core::ffi::{CStr, c_char, c_void};
use core::ptr::{null, null_mut};

use imgui::{DrawCmd, DrawCmdParams, DrawData, DrawIdx, DrawVert, TextureId};
use rex_com::com_call;
use rex_com::*;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

const VERTEX_SHADER: &CStr = c"
cbuffer vertexBuffer : register(b0)
{
    float4x4 ProjectionMatrix;
};
struct VS_INPUT
{
    float2 pos : POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};
struct PS_INPUT
{
    float4 pos : SV_POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};
PS_INPUT main(VS_INPUT input)
{
    PS_INPUT output;
    output.pos = mul(ProjectionMatrix, float4(input.pos.xy, 0.f, 1.f));
    output.col = input.col;
    output.uv  = input.uv;
    return output;
}";

const PIXEL_SHADER: &CStr = c"
struct PS_INPUT
{
    float4 pos : SV_POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};
sampler sampler0;
Texture2D texture0;
float4 main(PS_INPUT input) : SV_Target
{
    float4 out_col = input.col * texture0.Sample(sampler0, input.uv);
    return out_col;
}";

type D3DCompileFn = unsafe extern "system" fn(
    src_data: *const c_void,
    src_data_size: usize,
    source_name: *const c_char,
    defines: *const c_void,
    include: *mut c_void,
    entrypoint: *const c_char,
    target: *const c_char,
    flags1: u32,
    flags2: u32,
    code: *mut *mut ID3DBlob,
    error_msgs: *mut *mut ID3DBlob,
) -> HRESULT;

/// `D3DCompile` from `d3dcompiler_47.dll` (resolved at runtime; `windows-sys`
/// has no Direct3D bindings).
fn d3d_compile(source: &CStr, target: &CStr) -> Result<*mut ID3DBlob, String> {
    unsafe {
        let module = LoadLibraryA(c"d3dcompiler_47.dll".as_ptr().cast());
        if module.is_null() {
            return Err("d3dcompiler_47.dll could not be loaded".into());
        }
        let Some(proc) = GetProcAddress(module, c"D3DCompile".as_ptr().cast()) else {
            return Err("D3DCompile not found".into());
        };
        let compile: D3DCompileFn = core::mem::transmute(proc);

        let mut blob = null_mut();
        let mut errors = null_mut();
        let hr = compile(
            source.as_ptr().cast(),
            source.to_bytes().len(),
            null(),
            null(),
            null_mut(),
            c"main".as_ptr(),
            target.as_ptr(),
            0,
            0,
            &mut blob,
            &mut errors,
        );
        if !succeeded(hr) {
            let message = if errors.is_null() {
                format!("D3DCompile failed: {hr:#X}")
            } else {
                let text = String::from_utf8_lossy(ID3DBlob::as_bytes(errors)).into_owned();
                com_release(errors);
                text
            };
            return Err(message);
        }
        Ok(blob)
    }
}

#[repr(C)]
struct VertexConstantBuffer {
    mvp: [[f32; 4]; 4],
}

/// Owned COM pointer; released on drop.
struct Com<T>(*mut T);

impl<T> Com<T> {
    const fn null() -> Self {
        Self(null_mut())
    }
}

impl<T> Drop for Com<T> {
    fn drop(&mut self) {
        unsafe { com_release(self.0) };
    }
}

pub struct Renderer {
    device: *mut ID3D11Device,
    context: *mut ID3D11DeviceContext,
    vertex_buffer: Com<ID3D11Buffer>,
    index_buffer: Com<ID3D11Buffer>,
    vertex_buffer_size: usize,
    index_buffer_size: usize,
    vertex_shader: Com<ID3D11VertexShader>,
    input_layout: Com<ID3D11InputLayout>,
    constant_buffer: Com<ID3D11Buffer>,
    pixel_shader: Com<ID3D11PixelShader>,
    font_sampler: Com<ID3D11SamplerState>,
    font_texture_view: Com<ID3D11ShaderResourceView>,
    rasterizer_state: Com<ID3D11RasterizerState>,
    blend_state: Com<ID3D11BlendState>,
    depth_stencil_state: Com<ID3D11DepthStencilState>,
}

// The renderer is only touched from the render thread.
unsafe impl Send for Renderer {}

impl Renderer {
    /// C++ `ImGui_ImplDX11_Init` + `ImGui_ImplDX11_CreateDeviceObjects`.
    ///
    /// # Safety
    /// `device` / `context` must be the game's live D3D11 device and immediate context.
    pub unsafe fn new(
        ctx: &mut imgui::Context,
        device: *mut ID3D11Device,
        context: *mut ID3D11DeviceContext,
    ) -> Result<Self, String> {
        if device.is_null() || context.is_null() {
            return Err("null D3D11 device or context".into());
        }
        unsafe {
            com_add_ref(device);
            com_add_ref(context);
        }
        ctx.set_renderer_name(Some(String::from("wheeler_re-dx11")));
        ctx.io_mut().backend_flags |= imgui::BackendFlags::RENDERER_HAS_VTX_OFFSET;

        let mut this = Self {
            device,
            context,
            vertex_buffer: Com::null(),
            index_buffer: Com::null(),
            vertex_buffer_size: 5000,
            index_buffer_size: 10000,
            vertex_shader: Com::null(),
            input_layout: Com::null(),
            constant_buffer: Com::null(),
            pixel_shader: Com::null(),
            font_sampler: Com::null(),
            font_texture_view: Com::null(),
            rasterizer_state: Com::null(),
            blend_state: Com::null(),
            depth_stencil_state: Com::null(),
        };
        unsafe {
            this.create_device_objects()?;
            this.create_fonts_texture(ctx)?;
        }
        Ok(this)
    }

    unsafe fn check(hr: HRESULT, what: &str) -> Result<(), String> {
        if succeeded(hr) {
            Ok(())
        } else {
            Err(format!("{what} failed: {hr:#X}"))
        }
    }

    unsafe fn create_device_objects(&mut self) -> Result<(), String> {
        let device = self.device;
        unsafe {
            // Vertex shader + input layout
            let vs_blob = d3d_compile(VERTEX_SHADER, c"vs_4_0")?;
            let vs = ID3DBlob::as_bytes(vs_blob);
            let hr = com_call!(device, CreateVertexShader: fn(*const c_void, usize, *mut c_void, *mut *mut ID3D11VertexShader) -> HRESULT;
                vs.as_ptr().cast(), vs.len(), null_mut(), &mut self.vertex_shader.0);
            if let Err(e) = Self::check(hr, "CreateVertexShader") {
                com_release(vs_blob);
                return Err(e);
            }

            let layout = [
                D3D11_INPUT_ELEMENT_DESC {
                    semantic_name: c"POSITION".as_ptr(),
                    semantic_index: 0,
                    format: DXGI_FORMAT_R32G32_FLOAT,
                    input_slot: 0,
                    aligned_byte_offset: core::mem::offset_of!(DrawVert, pos) as u32,
                    input_slot_class: D3D11_INPUT_PER_VERTEX_DATA,
                    instance_data_step_rate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    semantic_name: c"TEXCOORD".as_ptr(),
                    semantic_index: 0,
                    format: DXGI_FORMAT_R32G32_FLOAT,
                    input_slot: 0,
                    aligned_byte_offset: core::mem::offset_of!(DrawVert, uv) as u32,
                    input_slot_class: D3D11_INPUT_PER_VERTEX_DATA,
                    instance_data_step_rate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    semantic_name: c"COLOR".as_ptr(),
                    semantic_index: 0,
                    format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    input_slot: 0,
                    aligned_byte_offset: core::mem::offset_of!(DrawVert, col) as u32,
                    input_slot_class: D3D11_INPUT_PER_VERTEX_DATA,
                    instance_data_step_rate: 0,
                },
            ];
            let hr = com_call!(device, CreateInputLayout: fn(*const D3D11_INPUT_ELEMENT_DESC, u32, *const c_void, usize, *mut *mut ID3D11InputLayout) -> HRESULT;
                layout.as_ptr(), layout.len() as u32, vs.as_ptr().cast(), vs.len(), &mut self.input_layout.0);
            com_release(vs_blob);
            Self::check(hr, "CreateInputLayout")?;

            let desc = D3D11_BUFFER_DESC {
                byte_width: core::mem::size_of::<VertexConstantBuffer>() as u32,
                usage: D3D11_USAGE_DYNAMIC,
                bind_flags: D3D11_BIND_CONSTANT_BUFFER,
                cpu_access_flags: D3D11_CPU_ACCESS_WRITE,
                ..Default::default()
            };
            let hr = com_call!(device, CreateBuffer: fn(*const D3D11_BUFFER_DESC, *const D3D11_SUBRESOURCE_DATA, *mut *mut ID3D11Buffer) -> HRESULT;
                &desc, null(), &mut self.constant_buffer.0);
            Self::check(hr, "CreateBuffer(constant)")?;

            // Pixel shader
            let ps_blob = d3d_compile(PIXEL_SHADER, c"ps_4_0")?;
            let ps = ID3DBlob::as_bytes(ps_blob);
            let hr = com_call!(device, CreatePixelShader: fn(*const c_void, usize, *mut c_void, *mut *mut ID3D11PixelShader) -> HRESULT;
                ps.as_ptr().cast(), ps.len(), null_mut(), &mut self.pixel_shader.0);
            com_release(ps_blob);
            Self::check(hr, "CreatePixelShader")?;

            // Blending
            let mut blend = D3D11_BLEND_DESC::default();
            blend.render_target[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                blend_enable: 1,
                src_blend: D3D11_BLEND_SRC_ALPHA,
                dest_blend: D3D11_BLEND_INV_SRC_ALPHA,
                blend_op: D3D11_BLEND_OP_ADD,
                src_blend_alpha: D3D11_BLEND_ONE,
                dest_blend_alpha: D3D11_BLEND_INV_SRC_ALPHA,
                blend_op_alpha: D3D11_BLEND_OP_ADD,
                render_target_write_mask: D3D11_COLOR_WRITE_ENABLE_ALL,
            };
            let hr = com_call!(device, CreateBlendState: fn(*const D3D11_BLEND_DESC, *mut *mut ID3D11BlendState) -> HRESULT;
                &blend, &mut self.blend_state.0);
            Self::check(hr, "CreateBlendState")?;

            // Rasterizer
            let raster = D3D11_RASTERIZER_DESC {
                fill_mode: D3D11_FILL_SOLID,
                cull_mode: D3D11_CULL_NONE,
                scissor_enable: 1,
                depth_clip_enable: 1,
                ..Default::default()
            };
            let hr = com_call!(device, CreateRasterizerState: fn(*const D3D11_RASTERIZER_DESC, *mut *mut ID3D11RasterizerState) -> HRESULT;
                &raster, &mut self.rasterizer_state.0);
            Self::check(hr, "CreateRasterizerState")?;

            // Depth-stencil
            let op = D3D11_DEPTH_STENCILOP_DESC {
                stencil_fail_op: D3D11_STENCIL_OP_KEEP,
                stencil_depth_fail_op: D3D11_STENCIL_OP_KEEP,
                stencil_pass_op: D3D11_STENCIL_OP_KEEP,
                stencil_func: D3D11_COMPARISON_ALWAYS,
            };
            let depth = D3D11_DEPTH_STENCIL_DESC {
                depth_enable: 0,
                depth_write_mask: D3D11_DEPTH_WRITE_MASK_ALL,
                depth_func: D3D11_COMPARISON_ALWAYS,
                stencil_enable: 0,
                front_face: op,
                back_face: op,
                ..Default::default()
            };
            let hr = com_call!(device, CreateDepthStencilState: fn(*const D3D11_DEPTH_STENCIL_DESC, *mut *mut ID3D11DepthStencilState) -> HRESULT;
                &depth, &mut self.depth_stencil_state.0);
            Self::check(hr, "CreateDepthStencilState")?;
        }
        Ok(())
    }

    unsafe fn create_fonts_texture(&mut self, ctx: &mut imgui::Context) -> Result<(), String> {
        let fonts = ctx.fonts();
        let texture = fonts.build_rgba32_texture();
        unsafe {
            self.font_texture_view.0 =
                create_texture_rgba(self.device, texture.data, texture.width, texture.height)?;

            let sampler = D3D11_SAMPLER_DESC {
                filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                address_u: D3D11_TEXTURE_ADDRESS_CLAMP,
                address_v: D3D11_TEXTURE_ADDRESS_CLAMP,
                address_w: D3D11_TEXTURE_ADDRESS_CLAMP,
                comparison_func: D3D11_COMPARISON_ALWAYS,
                ..Default::default()
            };
            let device = self.device;
            let hr = com_call!(device, CreateSamplerState: fn(*const D3D11_SAMPLER_DESC, *mut *mut ID3D11SamplerState) -> HRESULT;
                &sampler, &mut self.font_sampler.0);
            Self::check(hr, "CreateSamplerState")?;
        }
        fonts.tex_id = TextureId::new(self.font_texture_view.0 as usize);
        Ok(())
    }

    /// Re-uploads the font atlas (after fonts were added post-init).
    pub fn rebuild_fonts(&mut self, ctx: &mut imgui::Context) -> Result<(), String> {
        self.font_texture_view = Com::null();
        self.font_sampler = Com::null();
        unsafe { self.create_fonts_texture(ctx) }
    }

    unsafe fn ensure_buffer(&mut self, vertex: bool, needed: usize) -> Result<(), String> {
        let (buffer, size, stride, bind, extra) = if vertex {
            (
                &mut self.vertex_buffer,
                &mut self.vertex_buffer_size,
                core::mem::size_of::<DrawVert>(),
                D3D11_BIND_VERTEX_BUFFER,
                5000,
            )
        } else {
            (
                &mut self.index_buffer,
                &mut self.index_buffer_size,
                core::mem::size_of::<DrawIdx>(),
                D3D11_BIND_INDEX_BUFFER,
                10000,
            )
        };
        if !buffer.0.is_null() && *size >= needed {
            return Ok(());
        }
        *buffer = Com::null();
        *size = needed + extra;
        let desc = D3D11_BUFFER_DESC {
            byte_width: (*size * stride) as u32,
            usage: D3D11_USAGE_DYNAMIC,
            bind_flags: bind,
            cpu_access_flags: D3D11_CPU_ACCESS_WRITE,
            ..Default::default()
        };
        let device = self.device;
        let hr = com_call!(device, CreateBuffer: fn(*const D3D11_BUFFER_DESC, *const D3D11_SUBRESOURCE_DATA, *mut *mut ID3D11Buffer) -> HRESULT;
            &desc, null(), &mut buffer.0);
        unsafe { Self::check(hr, "CreateBuffer") }
    }

    unsafe fn setup_render_state(&self, draw_data: &DrawData) {
        let ctx = self.context;
        let viewport = D3D11_VIEWPORT {
            top_left_x: 0.0,
            top_left_y: 0.0,
            width: draw_data.display_size[0],
            height: draw_data.display_size[1],
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let stride = core::mem::size_of::<DrawVert>() as u32;
        let offset = 0u32;
        let blend_factor = [0.0f32; 4];
        let index_format = if core::mem::size_of::<DrawIdx>() == 2 {
            DXGI_FORMAT_R16_UINT
        } else {
            DXGI_FORMAT_R32_UINT
        };

        com_call!(ctx, RSSetViewports: fn(u32, *const D3D11_VIEWPORT); 1, &viewport);
        com_call!(ctx, IASetInputLayout: fn(*mut ID3D11InputLayout); self.input_layout.0);
        com_call!(ctx, IASetVertexBuffers: fn(u32, u32, *const *mut ID3D11Buffer, *const u32, *const u32);
            0, 1, &self.vertex_buffer.0, &stride, &offset);
        com_call!(ctx, IASetIndexBuffer: fn(*mut ID3D11Buffer, DXGI_FORMAT, u32); self.index_buffer.0, index_format, 0);
        com_call!(ctx, IASetPrimitiveTopology: fn(D3D11_PRIMITIVE_TOPOLOGY); D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        com_call!(ctx, VSSetShader: fn(*mut ID3D11VertexShader, *const *mut c_void, u32); self.vertex_shader.0, null(), 0);
        com_call!(ctx, VSSetConstantBuffers: fn(u32, u32, *const *mut ID3D11Buffer); 0, 1, &self.constant_buffer.0);
        com_call!(ctx, PSSetShader: fn(*mut ID3D11PixelShader, *const *mut c_void, u32); self.pixel_shader.0, null(), 0);
        com_call!(ctx, PSSetSamplers: fn(u32, u32, *const *mut ID3D11SamplerState); 0, 1, &self.font_sampler.0);
        com_call!(ctx, GSSetShader: fn(*mut c_void, *const *mut c_void, u32); null_mut(), null(), 0);
        com_call!(ctx, HSSetShader: fn(*mut c_void, *const *mut c_void, u32); null_mut(), null(), 0);
        com_call!(ctx, DSSetShader: fn(*mut c_void, *const *mut c_void, u32); null_mut(), null(), 0);
        com_call!(ctx, CSSetShader: fn(*mut c_void, *const *mut c_void, u32); null_mut(), null(), 0);
        com_call!(ctx, OMSetBlendState: fn(*mut ID3D11BlendState, *const f32, u32); self.blend_state.0, blend_factor.as_ptr(), 0xFFFF_FFFF);
        com_call!(ctx, OMSetDepthStencilState: fn(*mut ID3D11DepthStencilState, u32); self.depth_stencil_state.0, 0);
        com_call!(ctx, RSSetState: fn(*mut ID3D11RasterizerState); self.rasterizer_state.0);
    }

    /// C++ `ImGui_ImplDX11_RenderDrawData`
    pub fn render(&mut self, draw_data: &DrawData) -> Result<(), String> {
        // `draw_lists()` builds a slice from a null pointer when empty.
        if draw_data.display_size[0] <= 0.0
            || draw_data.display_size[1] <= 0.0
            || draw_data.draw_lists_count() == 0
        {
            return Ok(());
        }
        let ctx = self.context;
        unsafe {
            self.ensure_buffer(true, draw_data.total_vtx_count as usize)?;
            self.ensure_buffer(false, draw_data.total_idx_count as usize)?;

            // Upload vertex/index data into a single contiguous GPU buffer.
            let mut vtx_resource: D3D11_MAPPED_SUBRESOURCE = core::mem::zeroed();
            let mut idx_resource: D3D11_MAPPED_SUBRESOURCE = core::mem::zeroed();
            let hr = com_call!(ctx, Map: fn(*mut ID3D11Buffer, u32, D3D11_MAP, u32, *mut D3D11_MAPPED_SUBRESOURCE) -> HRESULT;
                self.vertex_buffer.0, 0, D3D11_MAP_WRITE_DISCARD, 0, &mut vtx_resource);
            Self::check(hr, "Map(vertex)")?;
            let hr = com_call!(ctx, Map: fn(*mut ID3D11Buffer, u32, D3D11_MAP, u32, *mut D3D11_MAPPED_SUBRESOURCE) -> HRESULT;
                self.index_buffer.0, 0, D3D11_MAP_WRITE_DISCARD, 0, &mut idx_resource);
            if let Err(e) = Self::check(hr, "Map(index)") {
                com_call!(ctx, Unmap: fn(*mut ID3D11Buffer, u32); self.vertex_buffer.0, 0);
                return Err(e);
            }
            let mut vtx_dst = vtx_resource.data.cast::<DrawVert>();
            let mut idx_dst = idx_resource.data.cast::<DrawIdx>();
            for list in draw_data.draw_lists() {
                let vtx = list.vtx_buffer();
                let idx = list.idx_buffer();
                core::ptr::copy_nonoverlapping(vtx.as_ptr(), vtx_dst, vtx.len());
                core::ptr::copy_nonoverlapping(idx.as_ptr(), idx_dst, idx.len());
                vtx_dst = vtx_dst.add(vtx.len());
                idx_dst = idx_dst.add(idx.len());
            }
            com_call!(ctx, Unmap: fn(*mut ID3D11Buffer, u32); self.vertex_buffer.0, 0);
            com_call!(ctx, Unmap: fn(*mut ID3D11Buffer, u32); self.index_buffer.0, 0);

            // Orthographic projection: top-left of the display is (0,0).
            let mut mapped: D3D11_MAPPED_SUBRESOURCE = core::mem::zeroed();
            let hr = com_call!(ctx, Map: fn(*mut ID3D11Buffer, u32, D3D11_MAP, u32, *mut D3D11_MAPPED_SUBRESOURCE) -> HRESULT;
                self.constant_buffer.0, 0, D3D11_MAP_WRITE_DISCARD, 0, &mut mapped);
            Self::check(hr, "Map(constant)")?;
            let [l, t] = draw_data.display_pos;
            let r = l + draw_data.display_size[0];
            let b = t + draw_data.display_size[1];
            mapped
                .data
                .cast::<VertexConstantBuffer>()
                .write(VertexConstantBuffer {
                    mvp: [
                        [2.0 / (r - l), 0.0, 0.0, 0.0],
                        [0.0, 2.0 / (t - b), 0.0, 0.0],
                        [0.0, 0.0, 0.5, 0.0],
                        [(r + l) / (l - r), (t + b) / (b - t), 0.5, 1.0],
                    ],
                });
            com_call!(ctx, Unmap: fn(*mut ID3D11Buffer, u32); self.constant_buffer.0, 0);

            let backup = StateBackup::capture(ctx);
            self.setup_render_state(draw_data);

            let clip_off = draw_data.display_pos;
            let mut global_vtx_offset = 0usize;
            let mut global_idx_offset = 0usize;
            for list in draw_data.draw_lists() {
                for cmd in list.commands() {
                    match cmd {
                        DrawCmd::Elements {
                            count,
                            cmd_params:
                                DrawCmdParams {
                                    clip_rect,
                                    texture_id,
                                    vtx_offset,
                                    idx_offset,
                                },
                        } => {
                            let rect = D3D11_RECT {
                                left: (clip_rect[0] - clip_off[0]) as i32,
                                top: (clip_rect[1] - clip_off[1]) as i32,
                                right: (clip_rect[2] - clip_off[0]) as i32,
                                bottom: (clip_rect[3] - clip_off[1]) as i32,
                            };
                            if rect.right <= rect.left || rect.bottom <= rect.top {
                                continue;
                            }
                            com_call!(ctx, RSSetScissorRects: fn(u32, *const D3D11_RECT); 1, &rect);
                            let srv = texture_id.id() as *mut ID3D11ShaderResourceView;
                            com_call!(ctx, PSSetShaderResources: fn(u32, u32, *const *mut ID3D11ShaderResourceView); 0, 1, &srv);
                            com_call!(ctx, DrawIndexed: fn(u32, u32, i32);
                                count as u32, (idx_offset + global_idx_offset) as u32, (vtx_offset + global_vtx_offset) as i32);
                        }
                        DrawCmd::ResetRenderState => self.setup_render_state(draw_data),
                        // Wheeler never registers draw callbacks, and imgui-rs does
                        // not expose the raw draw list needed to invoke one.
                        DrawCmd::RawCallback { .. } => {}
                    }
                }
                global_idx_offset += list.idx_buffer().len();
                global_vtx_offset += list.vtx_buffer().len();
            }

            backup.restore(ctx);
        }
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            com_release(self.context);
            com_release(self.device);
        }
    }
}

/// Creates an immutable RGBA8 texture and returns its shader resource view.
///
/// # Safety
/// `device` must be a live D3D11 device; `rgba` must hold `width * height * 4` bytes.
pub unsafe fn create_texture_rgba(
    device: *mut ID3D11Device,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<*mut ID3D11ShaderResourceView, String> {
    let desc = D3D11_TEXTURE2D_DESC {
        width,
        height,
        mip_levels: 1,
        array_size: 1,
        format: DXGI_FORMAT_R8G8B8A8_UNORM,
        sample_desc: DXGI_SAMPLE_DESC {
            count: 1,
            quality: 0,
        },
        usage: D3D11_USAGE_DEFAULT,
        bind_flags: D3D11_BIND_SHADER_RESOURCE,
        cpu_access_flags: 0,
        misc_flags: 0,
    };
    let data = D3D11_SUBRESOURCE_DATA {
        sys_mem: rgba.as_ptr().cast(),
        sys_mem_pitch: width * 4,
        sys_mem_slice_pitch: 0,
    };
    let mut texture: *mut ID3D11Texture2D = null_mut();
    let hr = com_call!(device, CreateTexture2D: fn(*const D3D11_TEXTURE2D_DESC, *const D3D11_SUBRESOURCE_DATA, *mut *mut ID3D11Texture2D) -> HRESULT;
        &desc, &data, &mut texture);
    if !succeeded(hr) {
        return Err(format!("CreateTexture2D failed: {hr:#X}"));
    }
    let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
        format: DXGI_FORMAT_R8G8B8A8_UNORM,
        view_dimension: D3D11_SRV_DIMENSION_TEXTURE2D,
        texture2d: D3D11_TEX2D_SRV {
            most_detailed_mip: 0,
            mip_levels: desc.mip_levels,
        },
        _union_tail: [0; 2],
    };
    let mut srv: *mut ID3D11ShaderResourceView = null_mut();
    let hr = com_call!(device, CreateShaderResourceView: fn(*mut ID3D11Texture2D, *const D3D11_SHADER_RESOURCE_VIEW_DESC, *mut *mut ID3D11ShaderResourceView) -> HRESULT;
        texture, &srv_desc, &mut srv);
    unsafe { com_release(texture) };
    if !succeeded(hr) {
        return Err(format!("CreateShaderResourceView failed: {hr:#X}"));
    }
    Ok(srv)
}

const MAX_RECTS: usize = D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE;

/// The pipeline state the game had before ImGui drew (C++ `BACKUP_DX11_STATE`).
struct StateBackup {
    scissor_rects_count: u32,
    viewports_count: u32,
    scissor_rects: [D3D11_RECT; MAX_RECTS],
    viewports: [D3D11_VIEWPORT; MAX_RECTS],
    rs: *mut ID3D11RasterizerState,
    blend_state: *mut ID3D11BlendState,
    blend_factor: [f32; 4],
    sample_mask: u32,
    stencil_ref: u32,
    depth_stencil_state: *mut ID3D11DepthStencilState,
    ps_shader_resource: *mut ID3D11ShaderResourceView,
    ps_sampler: *mut ID3D11SamplerState,
    ps: *mut c_void,
    vs: *mut c_void,
    gs: *mut c_void,
    ps_instances_count: u32,
    vs_instances_count: u32,
    gs_instances_count: u32,
    ps_instances: [*mut c_void; 256],
    vs_instances: [*mut c_void; 256],
    gs_instances: [*mut c_void; 256],
    primitive_topology: D3D11_PRIMITIVE_TOPOLOGY,
    index_buffer: *mut ID3D11Buffer,
    vertex_buffer: *mut ID3D11Buffer,
    vs_constant_buffer: *mut ID3D11Buffer,
    index_buffer_offset: u32,
    vertex_buffer_stride: u32,
    vertex_buffer_offset: u32,
    index_buffer_format: DXGI_FORMAT,
    input_layout: *mut ID3D11InputLayout,
}

impl StateBackup {
    unsafe fn capture(ctx: *mut ID3D11DeviceContext) -> Box<Self> {
        let mut s: Box<Self> = Box::new(unsafe { core::mem::zeroed() });
        s.scissor_rects_count = MAX_RECTS as u32;
        s.viewports_count = MAX_RECTS as u32;
        s.ps_instances_count = 256;
        s.vs_instances_count = 256;
        s.gs_instances_count = 256;
        com_call!(ctx, RSGetScissorRects: fn(*mut u32, *mut D3D11_RECT); &mut s.scissor_rects_count, s.scissor_rects.as_mut_ptr());
        com_call!(ctx, RSGetViewports: fn(*mut u32, *mut D3D11_VIEWPORT); &mut s.viewports_count, s.viewports.as_mut_ptr());
        com_call!(ctx, RSGetState: fn(*mut *mut ID3D11RasterizerState); &mut s.rs);
        com_call!(ctx, OMGetBlendState: fn(*mut *mut ID3D11BlendState, *mut f32, *mut u32); &mut s.blend_state, s.blend_factor.as_mut_ptr(), &mut s.sample_mask);
        com_call!(ctx, OMGetDepthStencilState: fn(*mut *mut ID3D11DepthStencilState, *mut u32); &mut s.depth_stencil_state, &mut s.stencil_ref);
        com_call!(ctx, PSGetShaderResources: fn(u32, u32, *mut *mut ID3D11ShaderResourceView); 0, 1, &mut s.ps_shader_resource);
        com_call!(ctx, PSGetSamplers: fn(u32, u32, *mut *mut ID3D11SamplerState); 0, 1, &mut s.ps_sampler);
        com_call!(ctx, PSGetShader: fn(*mut *mut c_void, *mut *mut c_void, *mut u32); &mut s.ps, s.ps_instances.as_mut_ptr(), &mut s.ps_instances_count);
        com_call!(ctx, VSGetShader: fn(*mut *mut c_void, *mut *mut c_void, *mut u32); &mut s.vs, s.vs_instances.as_mut_ptr(), &mut s.vs_instances_count);
        com_call!(ctx, VSGetConstantBuffers: fn(u32, u32, *mut *mut ID3D11Buffer); 0, 1, &mut s.vs_constant_buffer);
        com_call!(ctx, GSGetShader: fn(*mut *mut c_void, *mut *mut c_void, *mut u32); &mut s.gs, s.gs_instances.as_mut_ptr(), &mut s.gs_instances_count);
        com_call!(ctx, IAGetPrimitiveTopology: fn(*mut D3D11_PRIMITIVE_TOPOLOGY); &mut s.primitive_topology);
        com_call!(ctx, IAGetIndexBuffer: fn(*mut *mut ID3D11Buffer, *mut DXGI_FORMAT, *mut u32); &mut s.index_buffer, &mut s.index_buffer_format, &mut s.index_buffer_offset);
        com_call!(ctx, IAGetVertexBuffers: fn(u32, u32, *mut *mut ID3D11Buffer, *mut u32, *mut u32); 0, 1, &mut s.vertex_buffer, &mut s.vertex_buffer_stride, &mut s.vertex_buffer_offset);
        com_call!(ctx, IAGetInputLayout: fn(*mut *mut ID3D11InputLayout); &mut s.input_layout);
        s
    }

    /// Restores and releases everything `capture` acquired.
    unsafe fn restore(self: Box<Self>, ctx: *mut ID3D11DeviceContext) {
        let s = self;
        unsafe {
            com_call!(ctx, RSSetScissorRects: fn(u32, *const D3D11_RECT); s.scissor_rects_count, s.scissor_rects.as_ptr());
            com_call!(ctx, RSSetViewports: fn(u32, *const D3D11_VIEWPORT); s.viewports_count, s.viewports.as_ptr());
            com_call!(ctx, RSSetState: fn(*mut ID3D11RasterizerState); s.rs);
            com_release(s.rs);
            com_call!(ctx, OMSetBlendState: fn(*mut ID3D11BlendState, *const f32, u32); s.blend_state, s.blend_factor.as_ptr(), s.sample_mask);
            com_release(s.blend_state);
            com_call!(ctx, OMSetDepthStencilState: fn(*mut ID3D11DepthStencilState, u32); s.depth_stencil_state, s.stencil_ref);
            com_release(s.depth_stencil_state);
            com_call!(ctx, PSSetShaderResources: fn(u32, u32, *const *mut ID3D11ShaderResourceView); 0, 1, &s.ps_shader_resource);
            com_release(s.ps_shader_resource);
            com_call!(ctx, PSSetSamplers: fn(u32, u32, *const *mut ID3D11SamplerState); 0, 1, &s.ps_sampler);
            com_release(s.ps_sampler);
            com_call!(ctx, PSSetShader: fn(*mut c_void, *const *mut c_void, u32); s.ps, s.ps_instances.as_ptr(), s.ps_instances_count);
            com_release(s.ps);
            for &i in &s.ps_instances[..s.ps_instances_count as usize] {
                com_release(i);
            }
            com_call!(ctx, VSSetShader: fn(*mut c_void, *const *mut c_void, u32); s.vs, s.vs_instances.as_ptr(), s.vs_instances_count);
            com_release(s.vs);
            com_call!(ctx, VSSetConstantBuffers: fn(u32, u32, *const *mut ID3D11Buffer); 0, 1, &s.vs_constant_buffer);
            com_release(s.vs_constant_buffer);
            com_call!(ctx, GSSetShader: fn(*mut c_void, *const *mut c_void, u32); s.gs, s.gs_instances.as_ptr(), s.gs_instances_count);
            com_release(s.gs);
            for &i in &s.vs_instances[..s.vs_instances_count as usize] {
                com_release(i);
            }
            for &i in &s.gs_instances[..s.gs_instances_count as usize] {
                com_release(i);
            }
            com_call!(ctx, IASetPrimitiveTopology: fn(D3D11_PRIMITIVE_TOPOLOGY); s.primitive_topology);
            com_call!(ctx, IASetIndexBuffer: fn(*mut ID3D11Buffer, DXGI_FORMAT, u32); s.index_buffer, s.index_buffer_format, s.index_buffer_offset);
            com_release(s.index_buffer);
            com_call!(ctx, IASetVertexBuffers: fn(u32, u32, *const *mut ID3D11Buffer, *const u32, *const u32);
                0, 1, &s.vertex_buffer, &s.vertex_buffer_stride, &s.vertex_buffer_offset);
            com_release(s.vertex_buffer);
            com_call!(ctx, IASetInputLayout: fn(*mut ID3D11InputLayout); s.input_layout);
            com_release(s.input_layout);
        }
    }
}
