use std::io::Result;

#[osiris::main(scale = true, restart = true)]
async fn foo() -> Result<()> {
    osiris::task::yield_now().await;
    Ok(())
}

#[test]
fn bar() {
    foo();
}
