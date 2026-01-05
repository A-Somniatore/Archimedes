//! Routing benchmarks.
//!
//! Run with: `cargo bench -p archimedes-router`

use archimedes_router::{MethodRouter, Router};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use http::Method;

fn build_router(num_routes: usize) -> Router {
    let mut router = Router::new();

    // Add static routes
    for i in 0..num_routes / 3 {
        router.insert(
            &format!("/api/v1/resource{i}"),
            MethodRouter::new().get(format!("getResource{i}")),
        );
    }

    // Add param routes
    for i in 0..num_routes / 3 {
        router.insert(
            &format!("/api/v1/resource{i}/{{id}}"),
            MethodRouter::new().get(format!("getResourceById{i}")),
        );
    }

    // Add nested routes
    for i in 0..num_routes / 3 {
        router.insert(
            &format!("/api/v1/org/{{orgId}}/resource{i}/{{id}}"),
            MethodRouter::new().get(format!("getOrgResource{i}")),
        );
    }

    router
}

fn bench_static_match(c: &mut Criterion) {
    let router = build_router(100);

    c.bench_function("static_match", |b| {
        b.iter(|| {
            black_box(router.match_route(&Method::GET, "/api/v1/resource50"));
        });
    });
}

fn bench_param_match(c: &mut Criterion) {
    let router = build_router(100);

    c.bench_function("param_match", |b| {
        b.iter(|| {
            black_box(router.match_route(&Method::GET, "/api/v1/resource25/12345"));
        });
    });
}

fn bench_nested_param_match(c: &mut Criterion) {
    let router = build_router(100);

    c.bench_function("nested_param_match", |b| {
        b.iter(|| {
            black_box(router.match_route(
                &Method::GET,
                "/api/v1/org/acme-corp/resource10/12345",
            ));
        });
    });
}

fn bench_miss(c: &mut Criterion) {
    let router = build_router(100);

    c.bench_function("miss", |b| {
        b.iter(|| {
            black_box(router.match_route(&Method::GET, "/api/v1/nonexistent/path"));
        });
    });
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    for num_routes in [10, 50, 100, 500, 1000] {
        let router = build_router(num_routes);

        group.bench_with_input(
            BenchmarkId::new("static_match", num_routes),
            &num_routes,
            |b, &n| {
                let path = format!("/api/v1/resource{}", n / 6);
                b.iter(|| black_box(router.match_route(&Method::GET, &path)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("param_match", num_routes),
            &num_routes,
            |b, &n| {
                let path = format!("/api/v1/resource{}/12345", n / 6);
                b.iter(|| black_box(router.match_route(&Method::GET, &path)));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_static_match,
    bench_param_match,
    bench_nested_param_match,
    bench_miss,
    bench_scaling
);
criterion_main!(benches);
