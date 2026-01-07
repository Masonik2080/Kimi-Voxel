// ============================================
// Thread-Local Context - Контексты для потоков
// ============================================
//
// Thread-local storage для MeshingContext.
// Каждый рабочий поток Rayon получает свой экземпляр контекста.

use std::cell::RefCell;
use super::context::MeshingContext;

thread_local! {
    /// Thread-local контекст для генерации мешей
    static MESHING_CONTEXT: RefCell<MeshingContext> = RefCell::new(MeshingContext::new());
}

/// Выполняет функцию с thread-local контекстом
/// 
/// # Пример
/// ```ignore
/// let (vertices, indices) = with_meshing_context(|ctx| {
///     // использовать ctx для генерации меша
///     ctx.take_results()
/// });
/// ```
#[inline]
#[allow(dead_code)]
pub fn with_meshing_context<F, R>(f: F) -> R
where
    F: FnOnce(&mut MeshingContext) -> R,
{
    MESHING_CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        f(&mut ctx)
    })
}
