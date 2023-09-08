mod bindgen {
    use wasmtime::component::*;
    pub type HttpRequest = mycelia::execution::types::HttpRequest;
    const _: () = {
        if !(36 == <HttpRequest as wasmtime::component::ComponentType>::SIZE32) {
            ::core::panicking::panic(
                "assertion failed: 36 == <HttpRequest as wasmtime::component::ComponentType>::SIZE32",
            )
        }
        if !(4 == <HttpRequest as wasmtime::component::ComponentType>::ALIGN32) {
            ::core::panicking::panic(
                "assertion failed: 4 == <HttpRequest as wasmtime::component::ComponentType>::ALIGN32",
            )
        }
    };
    pub type HttpResponse = mycelia::execution::types::HttpResponse;
    const _: () = {
        if !(20 == <HttpResponse as wasmtime::component::ComponentType>::SIZE32) {
            ::core::panicking::panic(
                "assertion failed: 20 == <HttpResponse as wasmtime::component::ComponentType>::SIZE32",
            )
        }
        if !(4 == <HttpResponse as wasmtime::component::ComponentType>::ALIGN32) {
            ::core::panicking::panic(
                "assertion failed: 4 == <HttpResponse as wasmtime::component::ComponentType>::ALIGN32",
            )
        }
    };
    pub struct FunctionWorld {
        handle_request: wasmtime::component::Func,
    }
    const _: () = {
        use wasmtime::component::__internal::anyhow;
        impl FunctionWorld {
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: mycelia::execution::types::Host + Send,
                T: Send,
            {
                mycelia::execution::types::add_to_linker(linker, get)?;
                Ok(())
            }
            /// Instantiates the provided `module` using the specified
            /// parameters, wrapping up the result in a structure that
            /// translates between wasm and the host.
            pub async fn instantiate_async<T: Send>(
                mut store: impl wasmtime::AsContextMut<Data = T>,
                component: &wasmtime::component::Component,
                linker: &wasmtime::component::Linker<T>,
            ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
                let instance = linker.instantiate_async(&mut store, component).await?;
                Ok((Self::new(store, &instance)?, instance))
            }
            /// Instantiates a pre-instantiated module using the specified
            /// parameters, wrapping up the result in a structure that
            /// translates between wasm and the host.
            pub async fn instantiate_pre<T: Send>(
                mut store: impl wasmtime::AsContextMut<Data = T>,
                instance_pre: &wasmtime::component::InstancePre<T>,
            ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
                let instance = instance_pre.instantiate_async(&mut store).await?;
                Ok((Self::new(store, &instance)?, instance))
            }
            /// Low-level creation wrapper for wrapping up the exports
            /// of the `instance` provided in this structure of wasm
            /// exports.
            ///
            /// This function will extract exports from the `instance`
            /// defined within `store` and wrap them all up in the
            /// returned structure which can be used to interact with
            /// the wasm module.
            pub fn new(
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Self> {
                let mut store = store.as_context_mut();
                let mut exports = instance.exports(&mut store);
                let mut __exports = exports.root();
                let handle_request = *__exports
                    .typed_func::<(&HttpRequest,), (HttpResponse,)>("handle-request")?
                    .func();
                Ok(FunctionWorld { handle_request })
            }
            pub async fn call_handle_request<S: wasmtime::AsContextMut>(
                &self,
                mut store: S,
                arg0: &HttpRequest,
            ) -> wasmtime::Result<HttpResponse>
            where
                <S as wasmtime::AsContext>::Data: Send,
            {
                let callee = unsafe {
                    wasmtime::component::TypedFunc::<
                        (&HttpRequest,),
                        (HttpResponse,),
                    >::new_unchecked(self.handle_request)
                };
                let (ret0,) = callee.call_async(store.as_context_mut(), (arg0,)).await?;
                callee.post_return_async(store.as_context_mut()).await?;
                Ok(ret0)
            }
        }
    };
    pub mod mycelia {
        pub mod execution {
            #[allow(clippy::all)]
            pub mod types {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub type Status = u16;
                const _: () = {
                    if !(2 == <Status as wasmtime::component::ComponentType>::SIZE32) {
                        ::core::panicking::panic(
                            "assertion failed: 2 == <Status as wasmtime::component::ComponentType>::SIZE32",
                        )
                    }
                    if !(2 == <Status as wasmtime::component::ComponentType>::ALIGN32) {
                        ::core::panicking::panic(
                            "assertion failed: 2 == <Status as wasmtime::component::ComponentType>::ALIGN32",
                        )
                    }
                };
                #[component(variant)]
                pub enum Method {
                    #[component(name = "get")]
                    Get,
                    #[component(name = "head")]
                    Head,
                    #[component(name = "post")]
                    Post,
                    #[component(name = "put")]
                    Put,
                    #[component(name = "delete")]
                    Delete,
                    #[component(name = "connect")]
                    Connect,
                    #[component(name = "options")]
                    Options,
                    #[component(name = "trace")]
                    Trace,
                    #[component(name = "patch")]
                    Patch,
                    #[component(name = "other")]
                    Other(String),
                }
                #[automatically_derived]
                impl ::core::clone::Clone for Method {
                    #[inline]
                    fn clone(&self) -> Method {
                        match self {
                            Method::Get => Method::Get,
                            Method::Head => Method::Head,
                            Method::Post => Method::Post,
                            Method::Put => Method::Put,
                            Method::Delete => Method::Delete,
                            Method::Connect => Method::Connect,
                            Method::Options => Method::Options,
                            Method::Trace => Method::Trace,
                            Method::Patch => Method::Patch,
                            Method::Other(__self_0) => {
                                Method::Other(::core::clone::Clone::clone(__self_0))
                            }
                        }
                    }
                }
                unsafe impl wasmtime::component::Lower for Method {
                    #[inline]
                    fn lower<T>(
                        &self,
                        cx: &mut wasmtime::component::__internal::LowerContext<'_, T>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        dst: &mut std::mem::MaybeUninit<Self::Lower>,
                    ) -> wasmtime::component::__internal::anyhow::Result<()> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Variant(
                                i,
                            ) => &cx.types[i],
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        match self {
                            Self::Get => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(0u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Get)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Head => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(1u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Head)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Post => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(2u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Post)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Put => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(3u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Put)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Delete => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(4u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Delete)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Connect => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(5u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Connect)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Options => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(6u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Options)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Trace => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(7u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Trace)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Patch => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(8u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Patch)
                                                }
                                            }
                                        },
                                        |dst| Ok(()),
                                    )
                                }
                            }
                            Self::Other(value) => {
                                {
                                    #[allow(unused_unsafe)]
                                    {
                                        unsafe {
                                            use ::wasmtime::component::__internal::MaybeUninitExt;
                                            let m: &mut std::mem::MaybeUninit<_> = dst;
                                            m.map(|p| &raw mut (*p).tag)
                                        }
                                    }
                                }
                                    .write(wasmtime::ValRaw::u32(9u32));
                                unsafe {
                                    wasmtime::component::__internal::lower_payload(
                                        {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = dst;
                                                    m.map(|p| &raw mut (*p).payload)
                                                }
                                            }
                                        },
                                        |payload| {
                                            #[allow(unused_unsafe)]
                                            {
                                                unsafe {
                                                    use ::wasmtime::component::__internal::MaybeUninitExt;
                                                    let m: &mut std::mem::MaybeUninit<_> = payload;
                                                    m.map(|p| &raw mut (*p).Other)
                                                }
                                            }
                                        },
                                        |dst| {
                                            value
                                                .lower(
                                                    cx,
                                                    ty
                                                        .cases[9usize]
                                                        .ty
                                                        .unwrap_or_else(
                                                            wasmtime::component::__internal::bad_type_info,
                                                        ),
                                                    dst,
                                                )
                                        },
                                    )
                                }
                            }
                        }
                    }
                    #[inline]
                    fn store<T>(
                        &self,
                        cx: &mut wasmtime::component::__internal::LowerContext<'_, T>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        mut offset: usize,
                    ) -> wasmtime::component::__internal::anyhow::Result<()> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Variant(
                                i,
                            ) => &cx.types[i],
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        if true {
                            if !(offset
                                % (<Self as wasmtime::component::ComponentType>::ALIGN32
                                    as usize) == 0)
                            {
                                ::core::panicking::panic(
                                    "assertion failed: offset % (<Self as wasmtime::component::ComponentType>::ALIGN32 as usize) == 0",
                                )
                            }
                        }
                        match self {
                            Self::Get => {
                                *cx.get::<1usize>(offset) = 0u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Head => {
                                *cx.get::<1usize>(offset) = 1u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Post => {
                                *cx.get::<1usize>(offset) = 2u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Put => {
                                *cx.get::<1usize>(offset) = 3u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Delete => {
                                *cx.get::<1usize>(offset) = 4u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Connect => {
                                *cx.get::<1usize>(offset) = 5u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Options => {
                                *cx.get::<1usize>(offset) = 6u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Trace => {
                                *cx.get::<1usize>(offset) = 7u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Patch => {
                                *cx.get::<1usize>(offset) = 8u8.to_le_bytes();
                                Ok(())
                            }
                            Self::Other(value) => {
                                *cx.get::<1usize>(offset) = 9u8.to_le_bytes();
                                value
                                    .store(
                                        cx,
                                        ty
                                            .cases[9usize]
                                            .ty
                                            .unwrap_or_else(
                                                wasmtime::component::__internal::bad_type_info,
                                            ),
                                        offset
                                            + <Self as wasmtime::component::__internal::ComponentVariant>::PAYLOAD_OFFSET32,
                                    )
                            }
                        }
                    }
                }
                unsafe impl wasmtime::component::Lift for Method {
                    #[inline]
                    fn lift(
                        cx: &mut wasmtime::component::__internal::LiftContext<'_>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        src: &Self::Lower,
                    ) -> wasmtime::component::__internal::anyhow::Result<Self> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Variant(
                                i,
                            ) => &cx.types[i],
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        Ok(
                            match src.tag.get_u32() {
                                0u32 => Self::Get,
                                1u32 => Self::Head,
                                2u32 => Self::Post,
                                3u32 => Self::Put,
                                4u32 => Self::Delete,
                                5u32 => Self::Connect,
                                6u32 => Self::Options,
                                7u32 => Self::Trace,
                                8u32 => Self::Patch,
                                9u32 => {
                                    Self::Other(
                                        <String as wasmtime::component::Lift>::lift(
                                            cx,
                                            ty
                                                .cases[9usize]
                                                .ty
                                                .unwrap_or_else(
                                                    wasmtime::component::__internal::bad_type_info,
                                                ),
                                            unsafe { &src.payload.Other },
                                        )?,
                                    )
                                }
                                discrim => {
                                    return ::anyhow::__private::Err(
                                        ::anyhow::Error::msg({
                                            let res = ::alloc::fmt::format(
                                                format_args!("unexpected discriminant: {0}", discrim),
                                            );
                                            res
                                        }),
                                    );
                                }
                            },
                        )
                    }
                    #[inline]
                    fn load(
                        cx: &mut wasmtime::component::__internal::LiftContext<'_>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        bytes: &[u8],
                    ) -> wasmtime::component::__internal::anyhow::Result<Self> {
                        let align = <Self as wasmtime::component::ComponentType>::ALIGN32;
                        if true {
                            if !((bytes.as_ptr() as usize) % (align as usize) == 0) {
                                ::core::panicking::panic(
                                    "assertion failed: (bytes.as_ptr() as usize) % (align as usize) == 0",
                                )
                            }
                        }
                        let discrim = bytes[0];
                        let payload_offset = <Self as wasmtime::component::__internal::ComponentVariant>::PAYLOAD_OFFSET32;
                        let payload = &bytes[payload_offset..];
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Variant(
                                i,
                            ) => &cx.types[i],
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        Ok(
                            match discrim {
                                0u8 => Self::Get,
                                1u8 => Self::Head,
                                2u8 => Self::Post,
                                3u8 => Self::Put,
                                4u8 => Self::Delete,
                                5u8 => Self::Connect,
                                6u8 => Self::Options,
                                7u8 => Self::Trace,
                                8u8 => Self::Patch,
                                9u8 => {
                                    Self::Other(
                                        <String as wasmtime::component::Lift>::load(
                                            cx,
                                            ty
                                                .cases[9usize]
                                                .ty
                                                .unwrap_or_else(
                                                    wasmtime::component::__internal::bad_type_info,
                                                ),
                                            &payload[..<String as wasmtime::component::ComponentType>::SIZE32],
                                        )?,
                                    )
                                }
                                discrim => {
                                    return ::anyhow::__private::Err(
                                        ::anyhow::Error::msg({
                                            let res = ::alloc::fmt::format(
                                                format_args!("unexpected discriminant: {0}", discrim),
                                            );
                                            res
                                        }),
                                    );
                                }
                            },
                        )
                    }
                }
                const _: () = {
                    #[doc(hidden)]
                    #[repr(C)]
                    pub struct LowerMethod<T9: Copy> {
                        tag: wasmtime::ValRaw,
                        payload: LowerPayloadMethod<T9>,
                    }
                    #[automatically_derived]
                    impl<T9: ::core::clone::Clone + Copy> ::core::clone::Clone
                    for LowerMethod<T9> {
                        #[inline]
                        fn clone(&self) -> LowerMethod<T9> {
                            LowerMethod {
                                tag: ::core::clone::Clone::clone(&self.tag),
                                payload: ::core::clone::Clone::clone(&self.payload),
                            }
                        }
                    }
                    #[automatically_derived]
                    impl<T9: ::core::marker::Copy + Copy> ::core::marker::Copy
                    for LowerMethod<T9> {}
                    #[doc(hidden)]
                    #[allow(non_snake_case)]
                    #[repr(C)]
                    union LowerPayloadMethod<T9: Copy> {
                        Get: [wasmtime::ValRaw; 0],
                        Head: [wasmtime::ValRaw; 0],
                        Post: [wasmtime::ValRaw; 0],
                        Put: [wasmtime::ValRaw; 0],
                        Delete: [wasmtime::ValRaw; 0],
                        Connect: [wasmtime::ValRaw; 0],
                        Options: [wasmtime::ValRaw; 0],
                        Trace: [wasmtime::ValRaw; 0],
                        Patch: [wasmtime::ValRaw; 0],
                        Other: T9,
                    }
                    #[automatically_derived]
                    #[allow(non_snake_case)]
                    impl<
                        T9: ::core::marker::Copy + ::core::clone::Clone + Copy,
                    > ::core::clone::Clone for LowerPayloadMethod<T9> {
                        #[inline]
                        fn clone(&self) -> LowerPayloadMethod<T9> {
                            let _: ::core::clone::AssertParamIsCopy<Self>;
                            *self
                        }
                    }
                    #[automatically_derived]
                    #[allow(non_snake_case)]
                    impl<T9: ::core::marker::Copy + Copy> ::core::marker::Copy
                    for LowerPayloadMethod<T9> {}
                    unsafe impl wasmtime::component::ComponentType for Method {
                        type Lower = LowerMethod<
                            <String as wasmtime::component::ComponentType>::Lower,
                        >;
                        #[inline]
                        fn typecheck(
                            ty: &wasmtime::component::__internal::InterfaceType,
                            types: &wasmtime::component::__internal::InstanceType<'_>,
                        ) -> wasmtime::component::__internal::anyhow::Result<()> {
                            wasmtime::component::__internal::typecheck_variant(
                                ty,
                                types,
                                &[
                                    ("get", None),
                                    ("head", None),
                                    ("post", None),
                                    ("put", None),
                                    ("delete", None),
                                    ("connect", None),
                                    ("options", None),
                                    ("trace", None),
                                    ("patch", None),
                                    (
                                        "other",
                                        Some(
                                            <String as wasmtime::component::ComponentType>::typecheck,
                                        ),
                                    ),
                                ],
                            )
                        }
                        const ABI: wasmtime::component::__internal::CanonicalAbiInfo = wasmtime::component::__internal::CanonicalAbiInfo::variant_static(
                            &[
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                Some(<String as wasmtime::component::ComponentType>::ABI),
                            ],
                        );
                    }
                    unsafe impl wasmtime::component::__internal::ComponentVariant
                    for Method {
                        const CASES: &'static [Option<
                            wasmtime::component::__internal::CanonicalAbiInfo,
                        >] = &[
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            Some(<String as wasmtime::component::ComponentType>::ABI),
                        ];
                    }
                };
                impl core::fmt::Debug for Method {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            Method::Get => f.debug_tuple("Method::Get").finish(),
                            Method::Head => f.debug_tuple("Method::Head").finish(),
                            Method::Post => f.debug_tuple("Method::Post").finish(),
                            Method::Put => f.debug_tuple("Method::Put").finish(),
                            Method::Delete => f.debug_tuple("Method::Delete").finish(),
                            Method::Connect => f.debug_tuple("Method::Connect").finish(),
                            Method::Options => f.debug_tuple("Method::Options").finish(),
                            Method::Trace => f.debug_tuple("Method::Trace").finish(),
                            Method::Patch => f.debug_tuple("Method::Patch").finish(),
                            Method::Other(e) => {
                                f.debug_tuple("Method::Other").field(e).finish()
                            }
                        }
                    }
                }
                const _: () = {
                    if !(12 == <Method as wasmtime::component::ComponentType>::SIZE32) {
                        ::core::panicking::panic(
                            "assertion failed: 12 == <Method as wasmtime::component::ComponentType>::SIZE32",
                        )
                    }
                    if !(4 == <Method as wasmtime::component::ComponentType>::ALIGN32) {
                        ::core::panicking::panic(
                            "assertion failed: 4 == <Method as wasmtime::component::ComponentType>::ALIGN32",
                        )
                    }
                };
                pub type Headers = Vec<(String, String)>;
                const _: () = {
                    if !(8 == <Headers as wasmtime::component::ComponentType>::SIZE32) {
                        ::core::panicking::panic(
                            "assertion failed: 8 == <Headers as wasmtime::component::ComponentType>::SIZE32",
                        )
                    }
                    if !(4 == <Headers as wasmtime::component::ComponentType>::ALIGN32) {
                        ::core::panicking::panic(
                            "assertion failed: 4 == <Headers as wasmtime::component::ComponentType>::ALIGN32",
                        )
                    }
                };
                pub type Body = Vec<u8>;
                const _: () = {
                    if !(8 == <Body as wasmtime::component::ComponentType>::SIZE32) {
                        ::core::panicking::panic(
                            "assertion failed: 8 == <Body as wasmtime::component::ComponentType>::SIZE32",
                        )
                    }
                    if !(4 == <Body as wasmtime::component::ComponentType>::ALIGN32) {
                        ::core::panicking::panic(
                            "assertion failed: 4 == <Body as wasmtime::component::ComponentType>::ALIGN32",
                        )
                    }
                };
                pub type Uri = String;
                const _: () = {
                    if !(8 == <Uri as wasmtime::component::ComponentType>::SIZE32) {
                        ::core::panicking::panic(
                            "assertion failed: 8 == <Uri as wasmtime::component::ComponentType>::SIZE32",
                        )
                    }
                    if !(4 == <Uri as wasmtime::component::ComponentType>::ALIGN32) {
                        ::core::panicking::panic(
                            "assertion failed: 4 == <Uri as wasmtime::component::ComponentType>::ALIGN32",
                        )
                    }
                };
                #[component(record)]
                pub struct HttpRequest {
                    #[component(name = "method")]
                    pub method: Method,
                    #[component(name = "headers")]
                    pub headers: Headers,
                    #[component(name = "body")]
                    pub body: Body,
                    #[component(name = "uri")]
                    pub uri: Uri,
                }
                #[automatically_derived]
                impl ::core::clone::Clone for HttpRequest {
                    #[inline]
                    fn clone(&self) -> HttpRequest {
                        HttpRequest {
                            method: ::core::clone::Clone::clone(&self.method),
                            headers: ::core::clone::Clone::clone(&self.headers),
                            body: ::core::clone::Clone::clone(&self.body),
                            uri: ::core::clone::Clone::clone(&self.uri),
                        }
                    }
                }
                unsafe impl wasmtime::component::Lower for HttpRequest {
                    #[inline]
                    fn lower<T>(
                        &self,
                        cx: &mut wasmtime::component::__internal::LowerContext<'_, T>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        dst: &mut std::mem::MaybeUninit<Self::Lower>,
                    ) -> wasmtime::component::__internal::anyhow::Result<()> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Record(i) => {
                                &cx.types[i]
                            }
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        wasmtime::component::Lower::lower(
                            &self.method,
                            cx,
                            ty.fields[0usize].ty,
                            {
                                #[allow(unused_unsafe)]
                                {
                                    unsafe {
                                        use ::wasmtime::component::__internal::MaybeUninitExt;
                                        let m: &mut std::mem::MaybeUninit<_> = dst;
                                        m.map(|p| &raw mut (*p).method)
                                    }
                                }
                            },
                        )?;
                        wasmtime::component::Lower::lower(
                            &self.headers,
                            cx,
                            ty.fields[1usize].ty,
                            {
                                #[allow(unused_unsafe)]
                                {
                                    unsafe {
                                        use ::wasmtime::component::__internal::MaybeUninitExt;
                                        let m: &mut std::mem::MaybeUninit<_> = dst;
                                        m.map(|p| &raw mut (*p).headers)
                                    }
                                }
                            },
                        )?;
                        wasmtime::component::Lower::lower(
                            &self.body,
                            cx,
                            ty.fields[2usize].ty,
                            {
                                #[allow(unused_unsafe)]
                                {
                                    unsafe {
                                        use ::wasmtime::component::__internal::MaybeUninitExt;
                                        let m: &mut std::mem::MaybeUninit<_> = dst;
                                        m.map(|p| &raw mut (*p).body)
                                    }
                                }
                            },
                        )?;
                        wasmtime::component::Lower::lower(
                            &self.uri,
                            cx,
                            ty.fields[3usize].ty,
                            {
                                #[allow(unused_unsafe)]
                                {
                                    unsafe {
                                        use ::wasmtime::component::__internal::MaybeUninitExt;
                                        let m: &mut std::mem::MaybeUninit<_> = dst;
                                        m.map(|p| &raw mut (*p).uri)
                                    }
                                }
                            },
                        )?;
                        Ok(())
                    }
                    #[inline]
                    fn store<T>(
                        &self,
                        cx: &mut wasmtime::component::__internal::LowerContext<'_, T>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        mut offset: usize,
                    ) -> wasmtime::component::__internal::anyhow::Result<()> {
                        if true {
                            if !(offset
                                % (<Self as wasmtime::component::ComponentType>::ALIGN32
                                    as usize) == 0)
                            {
                                ::core::panicking::panic(
                                    "assertion failed: offset % (<Self as wasmtime::component::ComponentType>::ALIGN32 as usize) == 0",
                                )
                            }
                        }
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Record(i) => {
                                &cx.types[i]
                            }
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        wasmtime::component::Lower::store(
                            &self.method,
                            cx,
                            ty.fields[0usize].ty,
                            <Method as wasmtime::component::ComponentType>::ABI
                                .next_field32_size(&mut offset),
                        )?;
                        wasmtime::component::Lower::store(
                            &self.headers,
                            cx,
                            ty.fields[1usize].ty,
                            <Headers as wasmtime::component::ComponentType>::ABI
                                .next_field32_size(&mut offset),
                        )?;
                        wasmtime::component::Lower::store(
                            &self.body,
                            cx,
                            ty.fields[2usize].ty,
                            <Body as wasmtime::component::ComponentType>::ABI
                                .next_field32_size(&mut offset),
                        )?;
                        wasmtime::component::Lower::store(
                            &self.uri,
                            cx,
                            ty.fields[3usize].ty,
                            <Uri as wasmtime::component::ComponentType>::ABI
                                .next_field32_size(&mut offset),
                        )?;
                        Ok(())
                    }
                }
                unsafe impl wasmtime::component::Lift for HttpRequest {
                    #[inline]
                    fn lift(
                        cx: &mut wasmtime::component::__internal::LiftContext<'_>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        src: &Self::Lower,
                    ) -> wasmtime::component::__internal::anyhow::Result<Self> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Record(i) => {
                                &cx.types[i]
                            }
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        Ok(Self {
                            method: <Method as wasmtime::component::Lift>::lift(
                                cx,
                                ty.fields[0usize].ty,
                                &src.method,
                            )?,
                            headers: <Headers as wasmtime::component::Lift>::lift(
                                cx,
                                ty.fields[1usize].ty,
                                &src.headers,
                            )?,
                            body: <Body as wasmtime::component::Lift>::lift(
                                cx,
                                ty.fields[2usize].ty,
                                &src.body,
                            )?,
                            uri: <Uri as wasmtime::component::Lift>::lift(
                                cx,
                                ty.fields[3usize].ty,
                                &src.uri,
                            )?,
                        })
                    }
                    #[inline]
                    fn load(
                        cx: &mut wasmtime::component::__internal::LiftContext<'_>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        bytes: &[u8],
                    ) -> wasmtime::component::__internal::anyhow::Result<Self> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Record(i) => {
                                &cx.types[i]
                            }
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        if true {
                            if !((bytes.as_ptr() as usize)
                                % (<Self as wasmtime::component::ComponentType>::ALIGN32
                                    as usize) == 0)
                            {
                                ::core::panicking::panic(
                                    "assertion failed: (bytes.as_ptr() as usize) %\\n        (<Self as wasmtime::component::ComponentType>::ALIGN32 as usize) == 0",
                                )
                            }
                        }
                        let mut offset = 0;
                        Ok(Self {
                            method: <Method as wasmtime::component::Lift>::load(
                                cx,
                                ty.fields[0usize].ty,
                                &bytes[<Method as wasmtime::component::ComponentType>::ABI
                                    .next_field32_size(
                                        &mut offset,
                                    )..][..<Method as wasmtime::component::ComponentType>::SIZE32],
                            )?,
                            headers: <Headers as wasmtime::component::Lift>::load(
                                cx,
                                ty.fields[1usize].ty,
                                &bytes[<Headers as wasmtime::component::ComponentType>::ABI
                                    .next_field32_size(
                                        &mut offset,
                                    )..][..<Headers as wasmtime::component::ComponentType>::SIZE32],
                            )?,
                            body: <Body as wasmtime::component::Lift>::load(
                                cx,
                                ty.fields[2usize].ty,
                                &bytes[<Body as wasmtime::component::ComponentType>::ABI
                                    .next_field32_size(
                                        &mut offset,
                                    )..][..<Body as wasmtime::component::ComponentType>::SIZE32],
                            )?,
                            uri: <Uri as wasmtime::component::Lift>::load(
                                cx,
                                ty.fields[3usize].ty,
                                &bytes[<Uri as wasmtime::component::ComponentType>::ABI
                                    .next_field32_size(
                                        &mut offset,
                                    )..][..<Uri as wasmtime::component::ComponentType>::SIZE32],
                            )?,
                        })
                    }
                }
                const _: () = {
                    #[doc(hidden)]
                    #[repr(C)]
                    pub struct LowerHttpRequest<T0: Copy, T1: Copy, T2: Copy, T3: Copy> {
                        method: T0,
                        headers: T1,
                        body: T2,
                        uri: T3,
                        _align: [wasmtime::ValRaw; 0],
                    }
                    #[automatically_derived]
                    impl<
                        T0: ::core::clone::Clone + Copy,
                        T1: ::core::clone::Clone + Copy,
                        T2: ::core::clone::Clone + Copy,
                        T3: ::core::clone::Clone + Copy,
                    > ::core::clone::Clone for LowerHttpRequest<T0, T1, T2, T3> {
                        #[inline]
                        fn clone(&self) -> LowerHttpRequest<T0, T1, T2, T3> {
                            LowerHttpRequest {
                                method: ::core::clone::Clone::clone(&self.method),
                                headers: ::core::clone::Clone::clone(&self.headers),
                                body: ::core::clone::Clone::clone(&self.body),
                                uri: ::core::clone::Clone::clone(&self.uri),
                                _align: ::core::clone::Clone::clone(&self._align),
                            }
                        }
                    }
                    #[automatically_derived]
                    impl<
                        T0: ::core::marker::Copy + Copy,
                        T1: ::core::marker::Copy + Copy,
                        T2: ::core::marker::Copy + Copy,
                        T3: ::core::marker::Copy + Copy,
                    > ::core::marker::Copy for LowerHttpRequest<T0, T1, T2, T3> {}
                    unsafe impl wasmtime::component::ComponentType for HttpRequest {
                        type Lower = LowerHttpRequest<
                            <Method as wasmtime::component::ComponentType>::Lower,
                            <Headers as wasmtime::component::ComponentType>::Lower,
                            <Body as wasmtime::component::ComponentType>::Lower,
                            <Uri as wasmtime::component::ComponentType>::Lower,
                        >;
                        const ABI: wasmtime::component::__internal::CanonicalAbiInfo = wasmtime::component::__internal::CanonicalAbiInfo::record_static(
                            &[
                                <Method as wasmtime::component::ComponentType>::ABI,
                                <Headers as wasmtime::component::ComponentType>::ABI,
                                <Body as wasmtime::component::ComponentType>::ABI,
                                <Uri as wasmtime::component::ComponentType>::ABI,
                            ],
                        );
                        #[inline]
                        fn typecheck(
                            ty: &wasmtime::component::__internal::InterfaceType,
                            types: &wasmtime::component::__internal::InstanceType<'_>,
                        ) -> wasmtime::component::__internal::anyhow::Result<()> {
                            wasmtime::component::__internal::typecheck_record(
                                ty,
                                types,
                                &[
                                    (
                                        "method",
                                        <Method as wasmtime::component::ComponentType>::typecheck,
                                    ),
                                    (
                                        "headers",
                                        <Headers as wasmtime::component::ComponentType>::typecheck,
                                    ),
                                    (
                                        "body",
                                        <Body as wasmtime::component::ComponentType>::typecheck,
                                    ),
                                    (
                                        "uri",
                                        <Uri as wasmtime::component::ComponentType>::typecheck,
                                    ),
                                ],
                            )
                        }
                    }
                };
                impl core::fmt::Debug for HttpRequest {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("HttpRequest")
                            .field("method", &self.method)
                            .field("headers", &self.headers)
                            .field("body", &self.body)
                            .field("uri", &self.uri)
                            .finish()
                    }
                }
                const _: () = {
                    if !(36
                        == <HttpRequest as wasmtime::component::ComponentType>::SIZE32)
                    {
                        ::core::panicking::panic(
                            "assertion failed: 36 == <HttpRequest as wasmtime::component::ComponentType>::SIZE32",
                        )
                    }
                    if !(4
                        == <HttpRequest as wasmtime::component::ComponentType>::ALIGN32)
                    {
                        ::core::panicking::panic(
                            "assertion failed: 4 == <HttpRequest as wasmtime::component::ComponentType>::ALIGN32",
                        )
                    }
                };
                #[component(record)]
                pub struct HttpResponse {
                    #[component(name = "status")]
                    pub status: Status,
                    #[component(name = "headers")]
                    pub headers: Headers,
                    #[component(name = "body")]
                    pub body: Body,
                }
                #[automatically_derived]
                impl ::core::clone::Clone for HttpResponse {
                    #[inline]
                    fn clone(&self) -> HttpResponse {
                        HttpResponse {
                            status: ::core::clone::Clone::clone(&self.status),
                            headers: ::core::clone::Clone::clone(&self.headers),
                            body: ::core::clone::Clone::clone(&self.body),
                        }
                    }
                }
                unsafe impl wasmtime::component::Lower for HttpResponse {
                    #[inline]
                    fn lower<T>(
                        &self,
                        cx: &mut wasmtime::component::__internal::LowerContext<'_, T>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        dst: &mut std::mem::MaybeUninit<Self::Lower>,
                    ) -> wasmtime::component::__internal::anyhow::Result<()> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Record(i) => {
                                &cx.types[i]
                            }
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        wasmtime::component::Lower::lower(
                            &self.status,
                            cx,
                            ty.fields[0usize].ty,
                            {
                                #[allow(unused_unsafe)]
                                {
                                    unsafe {
                                        use ::wasmtime::component::__internal::MaybeUninitExt;
                                        let m: &mut std::mem::MaybeUninit<_> = dst;
                                        m.map(|p| &raw mut (*p).status)
                                    }
                                }
                            },
                        )?;
                        wasmtime::component::Lower::lower(
                            &self.headers,
                            cx,
                            ty.fields[1usize].ty,
                            {
                                #[allow(unused_unsafe)]
                                {
                                    unsafe {
                                        use ::wasmtime::component::__internal::MaybeUninitExt;
                                        let m: &mut std::mem::MaybeUninit<_> = dst;
                                        m.map(|p| &raw mut (*p).headers)
                                    }
                                }
                            },
                        )?;
                        wasmtime::component::Lower::lower(
                            &self.body,
                            cx,
                            ty.fields[2usize].ty,
                            {
                                #[allow(unused_unsafe)]
                                {
                                    unsafe {
                                        use ::wasmtime::component::__internal::MaybeUninitExt;
                                        let m: &mut std::mem::MaybeUninit<_> = dst;
                                        m.map(|p| &raw mut (*p).body)
                                    }
                                }
                            },
                        )?;
                        Ok(())
                    }
                    #[inline]
                    fn store<T>(
                        &self,
                        cx: &mut wasmtime::component::__internal::LowerContext<'_, T>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        mut offset: usize,
                    ) -> wasmtime::component::__internal::anyhow::Result<()> {
                        if true {
                            if !(offset
                                % (<Self as wasmtime::component::ComponentType>::ALIGN32
                                    as usize) == 0)
                            {
                                ::core::panicking::panic(
                                    "assertion failed: offset % (<Self as wasmtime::component::ComponentType>::ALIGN32 as usize) == 0",
                                )
                            }
                        }
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Record(i) => {
                                &cx.types[i]
                            }
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        wasmtime::component::Lower::store(
                            &self.status,
                            cx,
                            ty.fields[0usize].ty,
                            <Status as wasmtime::component::ComponentType>::ABI
                                .next_field32_size(&mut offset),
                        )?;
                        wasmtime::component::Lower::store(
                            &self.headers,
                            cx,
                            ty.fields[1usize].ty,
                            <Headers as wasmtime::component::ComponentType>::ABI
                                .next_field32_size(&mut offset),
                        )?;
                        wasmtime::component::Lower::store(
                            &self.body,
                            cx,
                            ty.fields[2usize].ty,
                            <Body as wasmtime::component::ComponentType>::ABI
                                .next_field32_size(&mut offset),
                        )?;
                        Ok(())
                    }
                }
                unsafe impl wasmtime::component::Lift for HttpResponse {
                    #[inline]
                    fn lift(
                        cx: &mut wasmtime::component::__internal::LiftContext<'_>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        src: &Self::Lower,
                    ) -> wasmtime::component::__internal::anyhow::Result<Self> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Record(i) => {
                                &cx.types[i]
                            }
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        Ok(Self {
                            status: <Status as wasmtime::component::Lift>::lift(
                                cx,
                                ty.fields[0usize].ty,
                                &src.status,
                            )?,
                            headers: <Headers as wasmtime::component::Lift>::lift(
                                cx,
                                ty.fields[1usize].ty,
                                &src.headers,
                            )?,
                            body: <Body as wasmtime::component::Lift>::lift(
                                cx,
                                ty.fields[2usize].ty,
                                &src.body,
                            )?,
                        })
                    }
                    #[inline]
                    fn load(
                        cx: &mut wasmtime::component::__internal::LiftContext<'_>,
                        ty: wasmtime::component::__internal::InterfaceType,
                        bytes: &[u8],
                    ) -> wasmtime::component::__internal::anyhow::Result<Self> {
                        let ty = match ty {
                            wasmtime::component::__internal::InterfaceType::Record(i) => {
                                &cx.types[i]
                            }
                            _ => wasmtime::component::__internal::bad_type_info(),
                        };
                        if true {
                            if !((bytes.as_ptr() as usize)
                                % (<Self as wasmtime::component::ComponentType>::ALIGN32
                                    as usize) == 0)
                            {
                                ::core::panicking::panic(
                                    "assertion failed: (bytes.as_ptr() as usize) %\\n        (<Self as wasmtime::component::ComponentType>::ALIGN32 as usize) == 0",
                                )
                            }
                        }
                        let mut offset = 0;
                        Ok(Self {
                            status: <Status as wasmtime::component::Lift>::load(
                                cx,
                                ty.fields[0usize].ty,
                                &bytes[<Status as wasmtime::component::ComponentType>::ABI
                                    .next_field32_size(
                                        &mut offset,
                                    )..][..<Status as wasmtime::component::ComponentType>::SIZE32],
                            )?,
                            headers: <Headers as wasmtime::component::Lift>::load(
                                cx,
                                ty.fields[1usize].ty,
                                &bytes[<Headers as wasmtime::component::ComponentType>::ABI
                                    .next_field32_size(
                                        &mut offset,
                                    )..][..<Headers as wasmtime::component::ComponentType>::SIZE32],
                            )?,
                            body: <Body as wasmtime::component::Lift>::load(
                                cx,
                                ty.fields[2usize].ty,
                                &bytes[<Body as wasmtime::component::ComponentType>::ABI
                                    .next_field32_size(
                                        &mut offset,
                                    )..][..<Body as wasmtime::component::ComponentType>::SIZE32],
                            )?,
                        })
                    }
                }
                const _: () = {
                    #[doc(hidden)]
                    #[repr(C)]
                    pub struct LowerHttpResponse<T0: Copy, T1: Copy, T2: Copy> {
                        status: T0,
                        headers: T1,
                        body: T2,
                        _align: [wasmtime::ValRaw; 0],
                    }
                    #[automatically_derived]
                    impl<
                        T0: ::core::clone::Clone + Copy,
                        T1: ::core::clone::Clone + Copy,
                        T2: ::core::clone::Clone + Copy,
                    > ::core::clone::Clone for LowerHttpResponse<T0, T1, T2> {
                        #[inline]
                        fn clone(&self) -> LowerHttpResponse<T0, T1, T2> {
                            LowerHttpResponse {
                                status: ::core::clone::Clone::clone(&self.status),
                                headers: ::core::clone::Clone::clone(&self.headers),
                                body: ::core::clone::Clone::clone(&self.body),
                                _align: ::core::clone::Clone::clone(&self._align),
                            }
                        }
                    }
                    #[automatically_derived]
                    impl<
                        T0: ::core::marker::Copy + Copy,
                        T1: ::core::marker::Copy + Copy,
                        T2: ::core::marker::Copy + Copy,
                    > ::core::marker::Copy for LowerHttpResponse<T0, T1, T2> {}
                    unsafe impl wasmtime::component::ComponentType for HttpResponse {
                        type Lower = LowerHttpResponse<
                            <Status as wasmtime::component::ComponentType>::Lower,
                            <Headers as wasmtime::component::ComponentType>::Lower,
                            <Body as wasmtime::component::ComponentType>::Lower,
                        >;
                        const ABI: wasmtime::component::__internal::CanonicalAbiInfo = wasmtime::component::__internal::CanonicalAbiInfo::record_static(
                            &[
                                <Status as wasmtime::component::ComponentType>::ABI,
                                <Headers as wasmtime::component::ComponentType>::ABI,
                                <Body as wasmtime::component::ComponentType>::ABI,
                            ],
                        );
                        #[inline]
                        fn typecheck(
                            ty: &wasmtime::component::__internal::InterfaceType,
                            types: &wasmtime::component::__internal::InstanceType<'_>,
                        ) -> wasmtime::component::__internal::anyhow::Result<()> {
                            wasmtime::component::__internal::typecheck_record(
                                ty,
                                types,
                                &[
                                    (
                                        "status",
                                        <Status as wasmtime::component::ComponentType>::typecheck,
                                    ),
                                    (
                                        "headers",
                                        <Headers as wasmtime::component::ComponentType>::typecheck,
                                    ),
                                    (
                                        "body",
                                        <Body as wasmtime::component::ComponentType>::typecheck,
                                    ),
                                ],
                            )
                        }
                    }
                };
                impl core::fmt::Debug for HttpResponse {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("HttpResponse")
                            .field("status", &self.status)
                            .field("headers", &self.headers)
                            .field("body", &self.body)
                            .finish()
                    }
                }
                const _: () = {
                    if !(20
                        == <HttpResponse as wasmtime::component::ComponentType>::SIZE32)
                    {
                        ::core::panicking::panic(
                            "assertion failed: 20 == <HttpResponse as wasmtime::component::ComponentType>::SIZE32",
                        )
                    }
                    if !(4
                        == <HttpResponse as wasmtime::component::ComponentType>::ALIGN32)
                    {
                        ::core::panicking::panic(
                            "assertion failed: 4 == <HttpResponse as wasmtime::component::ComponentType>::ALIGN32",
                        )
                    }
                };
                pub trait Host {}
                pub fn add_to_linker<T, U>(
                    linker: &mut wasmtime::component::Linker<T>,
                    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                ) -> wasmtime::Result<()>
                where
                    T: Send,
                    U: Host + Send,
                {
                    let mut inst = linker.instance("mycelia:execution/types@0.0.1")?;
                    Ok(())
                }
            }
        }
    }
    const _: &str = "package mycelia:execution@0.0.1\n\n// ATTENTION :)\n// These are intended only to get mycelia to MVP.\n// Once https://github.com/WebAssembly/wasi-http matures\n// We MUST move towards adoping support.\n// This is non-negotiable.\n\ninterface types {\n  type status = u16\n  variant method {\n    get,\n    head,\n    post,\n    put,\n    delete,\n    connect,\n    options,\n    trace,\n    patch,\n    other(string)\n  }\n\n  type headers = list<tuple<string, string>>\n  type body = list<u8>\n  type uri = string\n\n  // Used for producing requests only\n  record options {\n    timeout-ms: option<u32>,\n  }\n\n  record http-request {\n    method: method,\n    headers: headers,\n    body: body,\n    uri: uri,\n  }\n\n  record http-response {\n    status: status,\n    headers: headers,\n    body: body,\n  }\n}\n\nworld function-world {\n  // todo.. these aren\'t real\n  use types.{http-request, http-response}\n\n\n  export handle-request: func(req: http-request) -> http-response\n}\n\n";
}
