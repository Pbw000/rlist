#[macro_export]
macro_rules! impl_storage_enum {
    // 带单个 extension 类型的版本（支持 driver 和 extension 的笛卡尔积组合）
    (
        $enum_name:ident: $error_type:ty,
        drivers: [
            $($variant:ident: $ty:ty),+ $(,)?
        ],
        extension: $ext:ty
    ) => {
        paste::paste! {
            // 生成枚举定义 - 包含普通驱动和 extension 扩展变体
            #[allow(non_camel_case_types)]
            pub enum $enum_name {
                // 普通驱动变体
                $($variant($ty),)+
                // 扩展变体：为每个 driver 生成与 extension 的组合
                $(
                    /// 带包装器的存储变体
                    [<$variant $ext>]($ext<$ty>),
                )+
            }

            // 生成 End2EndCopyMeta 枚举 - extension 包装器与原有类型使用相同的 Meta
            #[allow(non_camel_case_types)]
            #[derive(Debug)]
            pub enum [<$enum_name End2EndCopyMeta>] {
                $($variant(<$ty as $crate::Storage>::End2EndCopyMeta),)+
            }

            // 生成 End2EndMoveMeta 枚举
            #[allow(non_camel_case_types)]
            #[derive(Debug)]
            pub enum [<$enum_name End2EndMoveMeta>] {
                $($variant(<$ty as $crate::Storage>::End2EndMoveMeta),)+
            }

            // 生成 ConfigMeta 枚举 - 包含普通驱动和 extension 的配置
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, strum::EnumMessage, strum::EnumIter)]
            pub enum [<$enum_name ConfigMeta>] {
                // 普通驱动配置
                $($variant(<$ty as $crate::Storage>::ConfigMeta),)+
                // 扩展驱动配置
                $(
                    [<$variant $ext>](<$ext<$ty> as $crate::Storage>::ConfigMeta),
                )+
            }

            // 为 ConfigMeta 生成辅助方法
            impl [<$enum_name ConfigMeta>] {
                /// 获取驱动名称（用于 URL 和解析）
                #[allow(non_upper_case_globals)]
                pub fn driver_name(&self) -> &'static str {
                    match self {
                        $([<$enum_name ConfigMeta>]::$variant(_) => {
                            paste::paste! {
                                {
                                    const [<S_ $variant:lower>]: &str = stringify!($variant);
                                    [<S_ $variant:lower>]
                                }
                            }
                        },)+
                        $(
                            [<$enum_name ConfigMeta>]::[<$variant $ext>](_) => {
                                paste::paste! {
                                    {
                                        const [<S_ $variant:lower _partial>]: &str = concat!(stringify!($variant), "_partial");
                                        [<S_ $variant:lower _partial>]
                                    }
                                }
                            }
                        )+
                    }
                }

                /// 获取配置模板 JSON
                pub fn get_template_json(&self) -> serde_json::Value {
                    match self {
                        $([<$enum_name ConfigMeta>]::$variant(_) => {
                            serde_json::to_value(<$ty as $crate::Storage>::auth_template()).unwrap_or_default()
                        },)+
                        $(
                            [<$enum_name ConfigMeta>]::[<$variant $ext>](_) => {
                                serde_json::to_value(<$ext<$ty> as $crate::Storage>::auth_template()).unwrap_or_default()
                            }
                        )+
                    }
                }
            }

            // 手动实现 FromStr 以支持 driver_name() 的解析
            impl std::str::FromStr for [<$enum_name ConfigMeta>] {
                type Err = String;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    paste::paste! {
                        $(
                            if s == stringify!($variant) {
                                return Ok([<$enum_name ConfigMeta>]::$variant(
                                    <$ty as $crate::Storage>::ConfigMeta::default()
                                ));
                            }
                        )+
                        $(
                            if s == concat!(stringify!($variant), "_partial") {
                                return Ok([<$enum_name ConfigMeta>]::[<$variant $ext>](
                                    <$ext<$ty> as $crate::Storage>::ConfigMeta::default()
                                ));
                            }
                        )+
                        Err(format!("未知的存储驱动：{}", s))
                    }
                }
            }

            // Default 实现手动在外部添加，避免宏展开问题

            // 生成 From 实现（普通驱动）
            $(
                impl From<$ty> for $enum_name {
                    fn from(storage: $ty) -> Self {
                        $enum_name::$variant(storage)
                    }
                }
            )+

            // 生成 From 实现（extension 类型）
            $(
                impl From<$ext<$ty>> for $enum_name {
                    fn from(storage: $ext<$ty>) -> Self {
                        $enum_name::[<$variant $ext>](storage)
                    }
                }
            )+

            // 生成 Storage trait 实现
            impl $crate::Storage for $enum_name {
                type Error = $error_type;
                type End2EndCopyMeta = [<$enum_name End2EndCopyMeta>];
                type End2EndMoveMeta = [<$enum_name End2EndMoveMeta>];
                type ConfigMeta = [<$enum_name ConfigMeta>];

                fn hash(&self) -> u64 {
                    match self {
                        $($enum_name::$variant(driver) => <$ty as $crate::Storage>::hash(driver),)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => <$ext<$ty> as $crate::Storage>::hash(driver),
                        )+
                    }
                }

                async fn build_cache(&self, path: &str) -> Result<(), Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::build_cache(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::build_cache(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                fn name(&self) -> &str {
                    match self {
                        $($enum_name::$variant(driver) => <$ty as $crate::Storage>::name(driver),)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => <$ext<$ty> as $crate::Storage>::name(driver),
                        )+
                    }
                }

                fn driver_name(&self) -> &str {
                    match self {
                        $($enum_name::$variant(driver) => <$ty as $crate::Storage>::driver_name(driver),)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => <$ext<$ty> as $crate::Storage>::driver_name(driver),
                        )+
                    }
                }

                async fn handle_path(&self, path: &str) -> Result<$crate::FileMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::handle_path(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::handle_path(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn list_files(
                    &self,
                    path: &str,
                    page_size: u32,
                    cursor: Option<String>,
                ) -> Result<$crate::storage::model::FileList, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::list_files(driver, path, page_size, cursor).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::list_files(driver, path, page_size, cursor).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn get_meta(&self, path: &str) -> Result<$crate::FileMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::get_meta(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::get_meta(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn get_download_meta_by_path(&self, path: &str) -> Result<$crate::storage::file_meta::DownloadableMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::get_download_meta_by_path(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::get_download_meta_by_path(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn download_file(&self, path: &str) -> Result<Box<dyn $crate::storage::model::FileContent>, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::download_file(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::download_file(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn create_folder(&self, path: &str) -> Result<$crate::FileMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::create_folder(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::create_folder(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn delete(&self, path: &str) -> Result<(), Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::delete(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::delete(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn rename(&self, old_path: &str, new_name: &str) -> Result<(), Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::rename(driver, old_path, new_name).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::rename(driver, old_path, new_name).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn copy_end_to_end(&self, source_meta: Self::End2EndCopyMeta, dest_path: &str) -> Result<(), Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            let [<$enum_name End2EndCopyMeta>]::$variant(meta) = source_meta else {
                                return Err($crate::error::RlistError::Storage(
                                    $crate::error::StorageError::InvalidConfig("copy_end_to_end: driver and meta mismatch".to_string()),
                                ).into());
                            };
                            <$ty as $crate::Storage>::copy_end_to_end(driver, meta, dest_path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                // extension 包装器使用与原始驱动相同的 meta 类型
                                let [<$enum_name End2EndCopyMeta>]::$variant(meta) = source_meta else {
                                    return Err($crate::error::RlistError::Storage(
                                        $crate::error::StorageError::InvalidConfig("copy_end_to_end: driver and meta mismatch".to_string()),
                                    ).into());
                                };
                                <$ext<$ty> as $crate::Storage>::copy_end_to_end(driver, meta, dest_path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn gen_copy_meta(&self, path: &str) -> Result<Self::End2EndCopyMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::gen_copy_meta(driver, path).await
                                .map([<$enum_name End2EndCopyMeta>]::$variant)
                                .map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                // extension 包装器使用与原始驱动相同的 meta 类型
                                <$ext<$ty> as $crate::Storage>::gen_copy_meta(driver, path).await
                                    .map([<$enum_name End2EndCopyMeta>]::$variant)
                                    .map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn move_end_to_end(&self, source_meta: Self::End2EndMoveMeta, dest_path: &str) -> Result<(), Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            let [<$enum_name End2EndMoveMeta>]::$variant(meta) = source_meta else {
                                return Err($crate::error::RlistError::Storage(
                                    $crate::error::StorageError::InvalidConfig("move_end_to_end: driver and meta mismatch".to_string()),
                                ).into());
                            };
                            <$ty as $crate::Storage>::move_end_to_end(driver, meta, dest_path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                // extension 包装器使用与原始驱动相同的 meta 类型
                                let [<$enum_name End2EndMoveMeta>]::$variant(meta) = source_meta else {
                                    return Err($crate::error::RlistError::Storage(
                                        $crate::error::StorageError::InvalidConfig("move_end_to_end: driver and meta mismatch".to_string()),
                                    ).into());
                                };
                                <$ext<$ty> as $crate::Storage>::move_end_to_end(driver, meta, dest_path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn gen_move_meta(&self, path: &str) -> Result<Self::End2EndMoveMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::gen_move_meta(driver, path).await
                                .map([<$enum_name End2EndMoveMeta>]::$variant)
                                .map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                // extension 包装器使用与原始驱动相同的 meta 类型
                                <$ext<$ty> as $crate::Storage>::gen_move_meta(driver, path).await
                                    .map([<$enum_name End2EndMoveMeta>]::$variant)
                                    .map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn get_upload_info(&self, params: $crate::storage::model::UploadInfoParams) -> Result<$crate::storage::model::UploadInfo, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::get_upload_info(driver, params.clone()).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::get_upload_info(driver, params.clone()).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn complete_upload(
                    &self,
                    path: &str,
                    upload_id: &str,
                    file_id: &str,
                    content_hash: &str,
                ) -> Result<Option<$crate::FileMeta>, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::complete_upload(driver, path, upload_id, file_id, content_hash).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::complete_upload(driver, path, upload_id, file_id, content_hash).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
                    &self,
                    path: &str,
                    content: R,
                    param: $crate::storage::model::UploadInfoParams,
                ) -> Result<$crate::FileMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            let inner_param = $crate::storage::model::UploadInfoParams {
                                path: path.to_string(),
                                size: param.size,
                                hash: param.hash,
                            };
                            <$ty as $crate::Storage>::upload_file(driver, path, content, inner_param).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                let inner_param = $crate::storage::model::UploadInfoParams {
                                    path: path.to_string(),
                                    size: param.size,
                                    hash: param.hash,
                                };
                                <$ext<$ty> as $crate::Storage>::upload_file(driver, path, content, inner_param).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                fn from_auth_data(data: Self::ConfigMeta) -> Result<Self, Self::Error>
                where
                    Self: Sized,
                {
                    match data {
                        $([<$enum_name ConfigMeta>]::$variant(config) => {
                            <$ty>::from_auth_data(config).map_err(|e| Into::<$error_type>::into(e)).map($enum_name::$variant)
                        },)+
                        $(
                            [<$enum_name ConfigMeta>]::[<$variant $ext>](config) => {
                                <$ext<$ty>>::from_auth_data(config).map_err(|e| Into::<$error_type>::into(e)).map($enum_name::[<$variant $ext>])
                            }
                        )+
                    }
                }
                fn to_auth_data(&self) -> Self::ConfigMeta
                where
                    Self: Sized,
                {
                    match self {
                        $($enum_name::$variant(driver) => {
                            [<$enum_name ConfigMeta>]::$variant(<$ty as $crate::Storage>::to_auth_data(driver))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                [<$enum_name ConfigMeta>]::[<$variant $ext>](<$ext<$ty> as $crate::Storage>::to_auth_data(driver))
                            }
                        )+
                    }
                }
                fn auth_template() -> Self::ConfigMeta
                where
                    Self: Sized,
                {
                    Self::ConfigMeta::default()
                }
            }
        }
    };

    // 不带扩展的版本
    (
        $enum_name:ident: $error_type:ty,
        drivers: [
            $($variant:ident: $ty:ty),+ $(,)?
        ]
    ) => {
        // 生成枚举定义
        pub enum $enum_name {
            $($variant($ty),)+
        }

        // 生成 End2EndCopyMeta 枚举
        #[allow(non_camel_case_types)]
        pub enum [<$enum_name End2EndCopyMeta>] {
            $($variant(<$ty as $crate::Storage>::End2EndCopyMeta),)+
        }

        // 生成 End2EndMoveMeta 枚举
        #[allow(non_camel_case_types)]
        pub enum [<$enum_name End2EndMoveMeta>] {
            $($variant(<$ty as $crate::Storage>::End2EndMoveMeta),)+
        }

        // 生成 ConfigMeta 枚举
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, strum::EnumString, strum::EnumMessage, strum::EnumIter)]
        pub enum [<$enum_name ConfigMeta>] {
            $($variant(<$ty as $crate::Storage>::ConfigMeta),)+
        }

        // 为 ConfigMeta 生成辅助方法
        impl [<$enum_name ConfigMeta>] {
            /// 获取驱动名称（用于 URL 和解析）
            #[allow(non_upper_case_globals)]
            pub fn driver_name(&self) -> &'static str {
                match self {
                    $([<$enum_name ConfigMeta>]::$variant(_) => {
                        paste::paste! {
                            {
                                const [<S_ $variant:lower>]: &str = stringify!($variant);
                                [<S_ $variant:lower>]
                            }
                        }
                    },)+
                }
            }

            /// 获取配置模板 JSON
            pub fn get_template_json(&self) -> serde_json::Value {
                match self {
                    $([<$enum_name ConfigMeta>]::$variant(_) => {
                        serde_json::to_value(<$ty as $crate::Storage>::auth_template()).unwrap_or_default()
                    },)+
                }
            }
        }

        // Default 实现手动在外部添加，避免宏展开问题

        // 生成 From 实现
        $(
            impl From<$ty> for $enum_name {
                fn from(storage: $ty) -> Self {
                    $enum_name::$variant(storage)
                }
            }
        )+

        // 生成 Storage trait 实现
        impl $crate::Storage for $enum_name {
            type Error = $error_type;
            type End2EndCopyMeta = [<$enum_name End2EndCopyMeta>];
            type End2EndMoveMeta = [<$enum_name End2EndMoveMeta>];
            type ConfigMeta = [<$enum_name ConfigMeta>];

            fn hash(&self) -> u64 {
                match self {
                    $($enum_name::$variant(driver) => <$ty as $crate::Storage>::hash(driver),)+
                }
            }

            fn name(&self) -> &str {
                match self {
                    $($enum_name::$variant(driver) => <$ty as $crate::Storage>::name(driver),)+
                }
            }

            fn driver_name(&self) -> &str {
                match self {
                    $($enum_name::$variant(driver) => <$ty as $crate::Storage>::driver_name(driver),)+
                }
            }

            async fn build_cache(&self, path: &str) -> Result<(), Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::build_cache(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn handle_path(&self, path: &str) -> Result<$crate::FileMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::handle_path(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn list_files(
                &self,
                path: &str,
                page_size: u32,
                cursor: Option<String>,
            ) -> Result<$crate::storage::model::FileList, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::list_files(driver, path, page_size, cursor)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn get_meta(&self, path: &str) -> Result<$crate::FileMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::get_meta(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn get_download_meta_by_path(
                &self,
                path: &str,
            ) -> Result<$crate::storage::file_meta::DownloadableMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::get_download_meta_by_path(driver, path)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn download_file(&self, path: &str) -> Result<Box<dyn $crate::storage::model::FileContent>, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::download_file(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn create_folder(&self, path: &str) -> Result<$crate::FileMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::create_folder(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn delete(&self, path: &str) -> Result<(), Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::delete(driver, path).await.map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn rename(&self, old_path: &str, new_name: &str) -> Result<$crate::FileMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::rename(driver, old_path, new_name)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn copy_end_to_end(&self, source_meta: Self::End2EndCopyMeta, dest_path: &str) -> Result<(), Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        let [<$enum_name End2EndCopyMeta>]::$variant(meta) = source_meta else {
                            return Err($crate::error::RlistError::Storage(
                                $crate::error::StorageError::InvalidConfig("copy_end_to_end: driver and meta mismatch".to_string()),
                            ).into());
                        };
                        <$ty as $crate::Storage>::copy_end_to_end(driver, meta, dest_path)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn gen_copy_meta(&self, path: &str) -> Result<Self::End2EndCopyMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::gen_copy_meta(driver, path)
                            .await
                            .map([<$enum_name End2EndCopyMeta>]::$variant)
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn move_end_to_end(&self, source_meta: Self::End2EndMoveMeta, dest_path: &str) -> Result<(), Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        let [<$enum_name End2EndMoveMeta>]::$variant(meta) = source_meta else {
                            return Err($crate::error::RlistError::Storage(
                                $crate::error::StorageError::InvalidConfig("move_end_to_end: driver and meta mismatch".to_string()),
                            ).into());
                        };
                        <$ty as $crate::Storage>::move_end_to_end(driver, meta, dest_path)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn gen_move_meta(&self, path: &str) -> Result<Self::End2EndMoveMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::gen_move_meta(driver, path)
                            .await
                            .map([<$enum_name End2EndMoveMeta>]::$variant)
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn get_upload_info(&self, params: $crate::storage::model::UploadInfoParams) -> Result<$crate::storage::model::UploadInfo, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::get_upload_info(driver, params.clone())
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
                &self,
                path: &str,
                content: R,
                param: $crate::storage::model::UploadInfoParams,
            ) -> Result<$crate::FileMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::upload_file(driver, path, content, param)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            fn from_auth_data(data: Self::ConfigMeta) -> Result<Self, Self::Error>
            where
                Self: Sized,
            {
                match data {
                    $([<$enum_name ConfigMeta>]::$variant(config) => {
                        <$ty>::from_auth_data(config).map_err(|e| Into::<$error_type>::into(e)).map($enum_name::$variant)
                    },)+
                }
            }

            fn to_auth_data(&self) -> Self::ConfigMeta
            where
                Self: Sized,
            {
                match self {
                    $($enum_name::$variant(driver) => {
                        [<$enum_name ConfigMeta>]::$variant(<$ty as $crate::Storage>::to_auth_data(driver))
                    },)+
                }
            }

            fn auth_template() -> Self::ConfigMeta
            where
                Self: Sized,
            {
                Self::ConfigMeta::default()
            }
        }
    };
}
