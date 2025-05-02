use commonlibsse_ng::re::{NiMatrix3::NiMatrix3, NiPoint3::NiPoint3};

pub fn matrix_from_axis_angle(theta: f32, axis: NiPoint3) -> NiMatrix3 {
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();
    let a = axis;

    NiMatrix3 {
        entry: [
            [
                cos_theta + a.x * a.x * (1.0 - cos_theta),
                a.x * a.y * (1.0 - cos_theta) - a.z * sin_theta,
                a.x * a.z * (1.0 - cos_theta) + a.y * sin_theta,
            ],
            [
                a.y * a.x * (1.0 - cos_theta) + a.z * sin_theta,
                cos_theta + a.y * a.y * (1.0 - cos_theta),
                a.y * a.z * (1.0 - cos_theta) - a.x * sin_theta,
            ],
            [
                a.z * a.x * (1.0 - cos_theta) - a.y * sin_theta,
                a.z * a.y * (1.0 - cos_theta) + a.x * sin_theta,
                cos_theta + a.z * a.z * (1.0 - cos_theta),
            ],
        ],
    }
}
