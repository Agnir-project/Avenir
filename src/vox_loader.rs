use dot_vox::DotVoxData;
use generic_octree::Octree;

impl<T> Into<Octree<L, f32>> for DotVoxData {
    fn into(self) -> Octree<T, f32> {
        Octree::new(0.0);
    }
}
