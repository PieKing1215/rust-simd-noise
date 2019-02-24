extern crate simdeez;
use self::simdeez::*;
use super::*;
use crate::shared::*;
use std::f32;

const X_PRIME: i32 = 1619;
const Y_PRIME: i32 = 31337;
const Z_PRIME: i32 = 6971;
const CELL_DIVISOR: f32 = 2147483648.0;

#[inline(always)]
unsafe fn hash_2d<S: Simd>(seed: S::Vi32, x: S::Vi32, y: S::Vi32) -> S::Vi32 {
    let mut hash = S::xor_epi32(seed, S::mullo_epi32(S::set1_epi32(X_PRIME), x));
    hash = S::xor_epi32(hash, S::mullo_epi32(S::set1_epi32(Y_PRIME), y));
    hash = S::mullo_epi32(
        hash,
        S::mullo_epi32(hash, S::mullo_epi32(hash, S::set1_epi32(60493))),
    );
    S::xor_epi32(S::srai_epi32(hash, 13), hash)
}
#[inline(always)]
unsafe fn val_coord_2d<S: Simd>(seed: S::Vi32, x: S::Vi32, y: S::Vi32) -> S::Vf32 {
    let mut hash = S::xor_epi32(seed, S::mullo_epi32(S::set1_epi32(X_PRIME), x));
    hash = S::xor_epi32(hash, S::mullo_epi32(S::set1_epi32(Y_PRIME), y));
    hash = S::mullo_epi32(
        hash,
        S::mullo_epi32(hash, S::mullo_epi32(hash, S::set1_epi32(60493))),
    );
    S::div_ps(S::cvtepi32_ps(hash), S::set1_ps(CELL_DIVISOR))
}

#[inline(always)]
pub unsafe fn cellular_2d<S: Simd>(
    x: S::Vf32,
    y: S::Vf32,
    distance_function: CellDistanceFunction,
    return_type: CellReturnType,
    jitter: S::Vf32,
) -> S::Vf32 {
    let mut distance = S::set1_ps(999999.0);
    let mut cellValue = S::setzero_ps();

    let mut xc = S::sub_epi32(S::cvtps_epi32(x),S::set1_epi32(1));
    let mut ycBase = S::sub_epi32(S::cvtps_epi32(y),S::set1_epi32(1));

    let mut xcf = S::sub_ps(S::cvtepi32_ps(xc),x);
    let mut ycfBase = S::sub_ps(S::cvtepi32_ps(ycBase),y);

    xc = S::mullo_epi32(xc,S::set1_epi32(X_PRIME));
    ycBase = S::mullo_epi32(ycBase,S::set1_epi32(Y_PRIME));

    for xi in 0 .. 3 {
        let mut ycf = ycfBase;
        let mut yc = ycBase;
        for yi in 0 .. 3 {
            let hash = hash_2d::<S>(S::set1_epi32(1337), xc, yc);
            let mut xd = S::sub_ps(S::cvtepi32_ps(S::and_epi32(hash,S::set1_epi32(BIT_10_MASK))),S::set1_ps(511.5));
            let mut yd = S::sub_ps(S::cvtepi32_ps(S::and_epi32(S::srai_epi32(hash,10),S::set1_epi32(BIT_10_MASK))),S::set1_ps(511.5));
            let invMag = S::mul_ps(jitter,S::rsqrt_ps(S::fmadd_ps(xd,xd,S::mul_ps(yd,yd))));
            xd = S::fmadd_ps(xd,invMag,xcf);
            yd = S::fmadd_ps(yd,invMag,ycf);

            let newCellValue = S::mul_ps(S::set1_ps(HASH_2_FLOAT), S::cvtepi32_ps(hash));         
            let newDistance = 
                match distance_function {
                    CellDistanceFunction::Euclidean => {
                        S::fmadd_ps(xd,xd,S::mul_ps(yd,yd))
                    }
                    CellDistanceFunction::Manhattan => {
                        S::add_ps(S::abs_ps(xd),S::abs_ps(yd))                        
                    }
                    CellDistanceFunction::Natural => {
                        let euc = S::add_ps(S::abs_ps(xd),S::abs_ps(yd));
                        let man = S::add_ps(S::abs_ps(xd),S::abs_ps(yd));
                        S::add_ps(euc,man)
                    }
                };
            let closer = S::cmplt_ps(newDistance, distance);
            distance = S::min_ps(newDistance,distance);
            cellValue = S::blendv_ps(cellValue, newCellValue, closer);			

            ycf = S::add_ps(ycf,S::set1_ps(1.0));
            yc = S::add_epi32(yc,S::set1_epi32(Y_PRIME));
        } 
        xcf = S::add_ps(xcf,S::set1_ps(1.0));
        xc = S::add_epi32(xc,S::set1_epi32(X_PRIME));
    }
    cellValue
}


#[inline(always)]
pub unsafe fn new_cellular_2d<S: Simd>(
    x: S::Vf32,
    y: S::Vf32,
    distance_function: CellDistanceFunction,
    return_type: CellReturnType,
    jitter: S::Vf32,
) -> S::Vf32 {
    let xr = S::cvtps_epi32(S::round_ps(x));
    let yr = S::cvtps_epi32(S::round_ps(y));
    let mut distance = S::set1_ps(f32::MAX);
    let mut xc = S::set1_epi32(0);
    let mut yc = S::set1_epi32(0);
    match distance_function {
        CellDistanceFunction::Euclidean => {
            for xmod in -1..2 {
                let xi = S::add_epi32(xr, S::set1_epi32(xmod));
                let xisubx = S::sub_ps(S::cvtepi32_ps(xi), x);
                for ymod in -1..2 {
                    let yi = S::add_epi32(yr, S::set1_epi32(ymod));
                    let hi = S::and_epi32(
                        hash_2d::<S>(S::set1_epi32(1337), xi, yi),
                        S::set1_epi32(0xff),
                    );
                    let cellx = S::i32gather_ps(&CELL_2D_X, hi);
                    let celly = S::i32gather_ps(&CELL_2D_Y, hi);

                    let vx = S::add_ps(xisubx, S::mul_ps(cellx, jitter));

                    let vy = S::add_ps(S::sub_ps(S::cvtepi32_ps(yi), y), S::mul_ps(celly, jitter));
                    let new_dist = S::add_ps(S::mul_ps(vx, vx), S::mul_ps(vy, vy));
                    let cond = S::cmplt_ps(new_dist, distance);
                    distance = S::blendv_ps(distance, new_dist, cond);
                    xc = S::blendv_epi32(xc, xi, S::castps_epi32(cond));
                    yc = S::blendv_epi32(yc, yi, S::castps_epi32(cond));
                }
            }
        }
        CellDistanceFunction::Manhattan => {
            for xmod in -1..2 {
                let xi = S::add_epi32(xr, S::set1_epi32(xmod));
                let xisubx = S::sub_ps(S::cvtepi32_ps(xi), x);
                for ymod in -1..2 {
                    let yi = S::add_epi32(yr, S::set1_epi32(ymod));
                    let hi = S::and_epi32(
                        hash_2d::<S>(S::set1_epi32(1337), xi, yi),
                        S::set1_epi32(0xff),
                    );
                    let cellx = S::i32gather_ps(&CELL_2D_X, hi);
                    let celly = S::i32gather_ps(&CELL_2D_Y, hi);

                    let vx = S::add_ps(xisubx, S::mul_ps(cellx, jitter));

                    let vy = S::add_ps(S::sub_ps(S::cvtepi32_ps(yi), y), S::mul_ps(celly, jitter));
                    let new_dist = S::add_ps(S::abs_ps(vx), S::abs_ps(vy));
                    let cond = S::cmplt_ps(new_dist, distance);
                    distance = S::blendv_ps(distance, new_dist, cond);
                    xc = S::blendv_epi32(xc, xi, S::castps_epi32(cond));
                    yc = S::blendv_epi32(yc, yi, S::castps_epi32(cond));
                }
            }
        }
        CellDistanceFunction::Natural => {
            for xmod in -1..2 {
                let xi = S::add_epi32(xr, S::set1_epi32(xmod));
                let xisubx = S::sub_ps(S::cvtepi32_ps(xi), x);
                for ymod in -1..2 {
                    let yi = S::add_epi32(yr, S::set1_epi32(ymod));
                    let hi = S::and_epi32(
                        hash_2d::<S>(S::set1_epi32(1337), xi, yi),
                        S::set1_epi32(0xff),
                    );
                    let cellx = S::i32gather_ps(&CELL_2D_X, hi);
                    let celly = S::i32gather_ps(&CELL_2D_Y, hi);

                    let vx = S::add_ps(xisubx, S::mul_ps(cellx, jitter));

                    let vy = S::add_ps(S::sub_ps(S::cvtepi32_ps(yi), y), S::mul_ps(celly, jitter));
                    let new_dist = S::add_ps(
                        S::add_ps(S::abs_ps(vx), S::abs_ps(vy)),
                        S::add_ps(S::mul_ps(vx, vx), S::mul_ps(vy, vy)),
                    );
                    let cond = S::cmplt_ps(new_dist, distance);
                    distance = S::blendv_ps(distance, new_dist, cond);
                    xc = S::blendv_epi32(xc, xi, S::castps_epi32(cond));
                    yc = S::blendv_epi32(yc, yi, S::castps_epi32(cond));
                }
            }
        }
    }

    match return_type {
        CellReturnType::Distance => distance,
        CellReturnType::CellValue => val_coord_2d::<S>(S::set1_epi32(1337), xc, yc),
    }
}

#[inline(always)]
unsafe fn hash_3d<S: Simd>(seed: S::Vi32, x: S::Vi32, y: S::Vi32, z: S::Vi32) -> S::Vi32 {
    let mut hash = S::xor_epi32(seed, S::mullo_epi32(S::set1_epi32(X_PRIME), x));
    hash = S::xor_epi32(hash, S::mullo_epi32(S::set1_epi32(Y_PRIME), y));
    hash = S::xor_epi32(hash, S::mullo_epi32(S::set1_epi32(Z_PRIME), z));
    hash = S::mullo_epi32(
        hash,
        S::mullo_epi32(hash, S::mullo_epi32(hash, S::set1_epi32(60493))),
    );
    S::xor_epi32(S::srai_epi32(hash, 13), hash)
}
#[inline(always)]
unsafe fn val_coord_3d<S: Simd>(seed: S::Vi32, x: S::Vi32, y: S::Vi32, z: S::Vi32) -> S::Vf32 {
    let mut hash = S::xor_epi32(seed, S::mullo_epi32(S::set1_epi32(X_PRIME), x));
    hash = S::xor_epi32(hash, S::mullo_epi32(S::set1_epi32(Y_PRIME), y));
    hash = S::xor_epi32(hash, S::mullo_epi32(S::set1_epi32(Z_PRIME), z));
    hash = S::mullo_epi32(
        hash,
        S::mullo_epi32(hash, S::mullo_epi32(hash, S::set1_epi32(60493))),
    );
    S::div_ps(S::cvtepi32_ps(hash), S::set1_ps(CELL_DIVISOR))
}
#[inline(always)]
pub unsafe fn cellular_3d<S: Simd>(
    x: S::Vf32,
    y: S::Vf32,
    z: S::Vf32,
    distance_function: CellDistanceFunction,
    return_type: CellReturnType,
    jitter: S::Vf32,
) -> S::Vf32 {
    let xr = S::cvtps_epi32(S::round_ps(x));
    let yr = S::cvtps_epi32(S::round_ps(y));
    let zr = S::cvtps_epi32(S::round_ps(z));
    let mut distance = S::set1_ps(f32::MAX);
    let mut xc = S::set1_epi32(0);
    let mut yc = S::set1_epi32(0);
    let mut zc = S::set1_epi32(0);

    match distance_function {
        CellDistanceFunction::Euclidean => {
            for xmod in -1..2 {
                let xi = S::add_epi32(xr, S::set1_epi32(xmod));
                let xisubx = S::sub_ps(S::cvtepi32_ps(xi), x);
                for ymod in -1..2 {
                    let yi = S::add_epi32(yr, S::set1_epi32(ymod));
                    for zmod in -1..2 {
                        let zi = S::add_epi32(zr, S::set1_epi32(zmod));
                        let hi = S::and_epi32(
                            hash_3d::<S>(S::set1_epi32(1337), xi, yi, zi),
                            S::set1_epi32(0xff),
                        );
                        let cellx = S::i32gather_ps(&CELL_3D_X, hi);
                        let celly = S::i32gather_ps(&CELL_3D_Y, hi);
                        let cellz = S::i32gather_ps(&CELL_3D_Z, hi);

                        let vx = S::add_ps(xisubx, S::mul_ps(cellx, jitter));

                        let vy =
                            S::add_ps(S::sub_ps(S::cvtepi32_ps(yi), y), S::mul_ps(celly, jitter));
                        let vz =
                            S::add_ps(S::sub_ps(S::cvtepi32_ps(zi), z), S::mul_ps(cellz, jitter));

                        let new_dist = S::add_ps(
                            S::mul_ps(vz, vz),
                            S::add_ps(S::mul_ps(vx, vx), S::mul_ps(vy, vy)),
                        );
                        let cond = S::cmplt_ps(new_dist, distance);
                        distance = S::blendv_ps(distance, new_dist, cond);
                        xc = S::blendv_epi32(xc, xi, S::castps_epi32(cond));
                        yc = S::blendv_epi32(yc, yi, S::castps_epi32(cond));
                        zc = S::blendv_epi32(zc, zi, S::castps_epi32(cond));
                    }
                }
            }
        }
        CellDistanceFunction::Manhattan => {
            for xmod in -1..2 {
                let xi = S::add_epi32(xr, S::set1_epi32(xmod));
                let xisubx = S::sub_ps(S::cvtepi32_ps(xi), x);
                for ymod in -1..2 {
                    let yi = S::add_epi32(yr, S::set1_epi32(ymod));
                    for zmod in -1..2 {
                        let zi = S::add_epi32(zr, S::set1_epi32(zmod));
                        let hi = S::and_epi32(
                            hash_3d::<S>(S::set1_epi32(1337), xi, yi, zi),
                            S::set1_epi32(0xff),
                        );
                        let cellx = S::i32gather_ps(&CELL_3D_X, hi);
                        let celly = S::i32gather_ps(&CELL_3D_Y, hi);
                        let cellz = S::i32gather_ps(&CELL_3D_Z, hi);

                        let vx = S::add_ps(xisubx, S::mul_ps(cellx, jitter));

                        let vy =
                            S::add_ps(S::sub_ps(S::cvtepi32_ps(yi), y), S::mul_ps(celly, jitter));
                        let vz =
                            S::add_ps(S::sub_ps(S::cvtepi32_ps(zi), z), S::mul_ps(cellz, jitter));
                        let new_dist =
                            S::add_ps(S::abs_ps(vz), S::add_ps(S::abs_ps(vx), S::abs_ps(vy)));
                        let cond = S::cmplt_ps(new_dist, distance);
                        distance = S::blendv_ps(distance, new_dist, cond);
                        xc = S::blendv_epi32(xc, xi, S::castps_epi32(cond));
                        yc = S::blendv_epi32(yc, yi, S::castps_epi32(cond));
                        zc = S::blendv_epi32(zc, zi, S::castps_epi32(cond));
                    }
                }
            }
        }
        CellDistanceFunction::Natural => {
            for xmod in -1..2 {
                let xi = S::add_epi32(xr, S::set1_epi32(xmod));
                let xisubx = S::sub_ps(S::cvtepi32_ps(xi), x);
                for ymod in -1..2 {
                    let yi = S::add_epi32(yr, S::set1_epi32(ymod));
                    for zmod in -1..2 {
                        let zi = S::add_epi32(zr, S::set1_epi32(zmod));

                        let hi = S::and_epi32(
                            hash_3d::<S>(S::set1_epi32(1337), xi, yi, zi),
                            S::set1_epi32(0xff),
                        );
                        let cellx = S::i32gather_ps(&CELL_3D_X, hi);
                        let celly = S::i32gather_ps(&CELL_3D_Y, hi);
                        let cellz = S::i32gather_ps(&CELL_3D_Z, hi);

                        let vx = S::add_ps(xisubx, S::mul_ps(cellx, jitter));

                        let vy =
                            S::add_ps(S::sub_ps(S::cvtepi32_ps(yi), y), S::mul_ps(celly, jitter));
                        let vz =
                            S::add_ps(S::sub_ps(S::cvtepi32_ps(zi), z), S::mul_ps(cellz, jitter));
                        let new_dist = S::add_ps(
                            S::add_ps(S::abs_ps(vz), S::add_ps(S::abs_ps(vx), S::abs_ps(vy))),
                            S::add_ps(
                                S::mul_ps(vz, vz),
                                S::add_ps(S::mul_ps(vx, vx), S::mul_ps(vy, vy)),
                            ),
                        );
                        let cond = S::cmplt_ps(new_dist, distance);
                        distance = S::blendv_ps(distance, new_dist, cond);
                        xc = S::blendv_epi32(xc, xi, S::castps_epi32(cond));
                        yc = S::blendv_epi32(yc, yi, S::castps_epi32(cond));
                        zc = S::blendv_epi32(zc, zi, S::castps_epi32(cond));
                    }
                }
            }
        }
    }

    match return_type {
        CellReturnType::Distance => distance,
        CellReturnType::CellValue => val_coord_3d::<S>(S::set1_epi32(1337), xc, yc, zc),
    }
}
