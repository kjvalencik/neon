use neon::prelude::*;

pub struct User {
  id: i32,
  first_name: String,
  last_name: String,
  email: String,
}

type Unit = ();

declare_types! {
  pub class JsPanickyAllocator for Unit {
    init(_) {
      panic!("allocator panicking")
    }
  }

  pub class JsPanickyConstructor for Unit {
    init(_) {
      Ok(())
    }

    call(_) {
      panic!("constructor call panicking")
    }

    constructor(_) {
      panic!("constructor panicking")
    }
  }

  pub class JsUser for User {
    init(cx) {
      let (cx, id) = cx.argument::<JsNumber>(0)?;
      let (cx, first_name) = cx.argument::<JsString>(1)?;
      let (cx, last_name) = cx.argument::<JsString>(2)?;
      let (_, email) = cx.argument::<JsString>(3)?;

      Ok(User {
        id: id.value() as i32,
        first_name: first_name.value(),
        last_name: last_name.value(),
        email: email.value(),
      })
    }

    method get(cx) {
      let (mut cx, attr) = cx.argument::<JsString>(0)?;
      let attr = attr.value();

      let this = cx.this();

      match &attr[..] {
        "id" => {
          let id = {
            let guard = cx.lock();
            let user = this.borrow(&guard);
            user.id
          };
          Ok(cx.number(id).upcast())
        },
        "first_name" => {
          let first_name = {
            let guard = cx.lock();
            let user = this.borrow(&guard);
            user.first_name.clone()
          };
          Ok(cx.string(&first_name).upcast())
        },
        "last_name" => {
          let last_name = {
            let guard = cx.lock();
            let user = this.borrow(&guard);
            user.last_name.clone()
          };
          Ok(cx.string(&last_name).upcast())
        },
        "email" => {
          let email = {
            let guard = cx.lock();
            let user = this.borrow(&guard);
            user.email.clone()
          };
          Ok(cx.string(&email).upcast())
        },
        _ => cx.throw_type_error("property does not exist")
      }
    }

    method panic(_) {
      panic!("User.prototype.panic")
    }
  }
}
