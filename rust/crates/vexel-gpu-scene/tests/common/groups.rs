// Shared by parity.rs and gpu_parity.rs (include!): opacity-group scene helpers.
/// Opacity-group tree over the spec list: leaves are spec indices in order.
enum Node {
    Leaf(usize),
    Group(f64, Vec<Node>),
}

fn tree(rng: &mut Lcg, next: &mut usize, total: usize, depth: u32, max: u32) -> Vec<Node> {
    let mut nodes = Vec::new();
    let budget = 1 + (rng.next() * 6.0) as usize;
    while *next < total && nodes.len() < budget {
        if depth < max && rng.next() < 0.45 {
            let opacity = match (rng.next() * 8.0) as u32 {
                0 => 1.0,
                1 => 0.1,
                _ => (rng.range(0.1, 1.0) * 100.0).round() / 100.0,
            };
            let children = tree(rng, next, total, depth + 1, max);
            nodes.push(Node::Group(opacity, children));
        } else {
            nodes.push(Node::Leaf(*next));
            *next += 1;
        }
    }
    nodes
}

fn group_tree(seed: u64, total: usize, max: u32) -> Vec<Node> {
    let mut rng = Lcg(seed ^ 0x9e37);
    let (mut next, mut out) = (0, Vec::new());
    while next < total {
        out.extend(tree(&mut rng, &mut next, total, 0, max));
    }
    out
}

fn gpu_nodes(specs: &[Spec], nodes: &[Node]) -> Vec<GpuNode> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Leaf(i) => GpuNode::Shape(GpuShape {
                edges: spec_edges(&specs[*i]),
                evenodd: specs[*i].evenodd,
                fill: Some(gpu_paint(&specs[*i])),
                stroke: None,
                clips: Vec::new(),
            }),
            Node::Group(opacity, c) => GpuNode::Group(GpuGroup {
                opacity: *opacity,
                clips: Vec::new(),
                children: gpu_nodes(specs, c),
            }),
        })
        .collect()
}

fn cpu_nodes<'a>(shapes: &[RenderShape<'a>], nodes: &[Node]) -> Vec<RenderNode<'a>> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Leaf(i) => RenderNode::Shape {
                shape: shapes[*i],
                clip: None,
            },
            Node::Group(opacity, c) => RenderNode::Group(RenderGroup {
                opacity: *opacity,
                clip: None,
                mask: None,
                children: Box::leak(cpu_nodes(shapes, c).into_boxed_slice()),
            }),
        })
        .collect()
}

fn group_scene(specs: &[Spec], nodes: &[Node], w: u32, h: u32) -> GpuScene {
    let mut scene = GpuScene::new(w, h);
    scene.nodes = gpu_nodes(specs, nodes);
    scene
}

fn group_cpu(specs: &[Spec], nodes: &[Node], w: u32, h: u32) -> Vec<u8> {
    let vb = ViewBox {
        x: 0.0,
        y: 0.0,
        width: f64::from(w),
        height: f64::from(h),
    };
    cpu_draw(specs, w, h, vb, (0.0, 0.0), 1.0, &|shapes, view, tile| {
        let tree = cpu_nodes(shapes, nodes);
        render_groups(&tree, view, tile, &|| false).unwrap()
    })
}
