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

                async fn build_cache(&self) -> Result<(), Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::build_cache(driver).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::build_cache(driver).await.map_err(|e| Into::<$error_type>::into(e))
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

                async fn rename(&self, old_path: &str, new_name: &str) -> Result<$crate::FileMeta, Self::Error> {
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

                async fn copy(&self, source_path: &str, dest_path: &str) -> Result<$crate::FileMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::copy(driver, source_path, dest_path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::copy(driver, source_path, dest_path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn move_(&self, source_path: &str, dest_path: &str) -> Result<$crate::FileMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::move_(driver, source_path, dest_path).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::move_(driver, source_path, dest_path).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                fn upload_mode(&self) -> $crate::storage::model::UploadMode {
                    match self {
                        $($enum_name::$variant(driver) => <$ty as $crate::Storage>::upload_mode(driver),)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => <$ext<$ty> as $crate::Storage>::upload_mode(driver),
                        )+
                    }
                }

                async fn get_upload_info(&self, path: &str, size: u64) -> Result<$crate::storage::model::UploadInfo, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::get_upload_info(driver, path, size).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::get_upload_info(driver, path, size).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                async fn upload_file(&self, path: &str, content: Vec<u8>) -> Result<$crate::FileMeta, Self::Error> {
                    match self {
                        $($enum_name::$variant(driver) => {
                            <$ty as $crate::Storage>::upload_file(driver, path, content).await.map_err(|e| Into::<$error_type>::into(e))
                        },)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => {
                                <$ext<$ty> as $crate::Storage>::upload_file(driver, path, content).await.map_err(|e| Into::<$error_type>::into(e))
                            },
                        )+
                    }
                }

                fn from_auth_data(json: &str) -> Result<Self, Self::Error>
                where
                    Self: Sized,
                {
                    $(
                        if let Ok(storage) = <$ty>::from_auth_data(json) {
                            return Ok($enum_name::$variant(storage));
                        }
                    )+
                    Err($crate::error::RlistError::Storage(
                        $crate::error::StorageError::InvalidConfig,
                    ))
                }

                fn auth_template(&self) -> String
                where
                    Self: Sized,
                {
                    match self {
                        $($enum_name::$variant(driver) => <$ty as $crate::Storage>::auth_template(driver),)+
                        $(
                            $enum_name::[<$variant $ext>](driver) => <$ext<$ty> as $crate::Storage>::auth_template(driver),
                        )+
                    }
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

            async fn build_cache(&self) -> Result<(), Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::build_cache(driver).await.map_err(|e| Into::<$error_type>::into(e))
                    },)+
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

            async fn copy(&self, source_path: &str, dest_path: &str) -> Result<$crate::FileMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::copy(driver, source_path, dest_path)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn move_(&self, source_path: &str, dest_path: &str) -> Result<$crate::FileMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::move_(driver, source_path, dest_path)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            fn upload_mode(&self) -> $crate::storage::model::UploadMode {
                match self {
                    $($enum_name::$variant(driver) => <$ty as $crate::Storage>::upload_mode(driver),)+
                }
            }

            async fn get_upload_info(&self, path: &str, size: u64) -> Result<$crate::storage::model::UploadInfo, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::get_upload_info(driver, path, size)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            async fn upload_file(&self, path: &str, content: Vec<u8>) -> Result<$crate::FileMeta, Self::Error> {
                match self {
                    $($enum_name::$variant(driver) => {
                        <$ty as $crate::Storage>::upload_file(driver, path, content)
                            .await
                            .map_err(|e| Into::<$error_type>::into(e))
                    },)+
                }
            }

            fn from_auth_data(json: &str) -> Result<Self, Self::Error>
            where
                Self: Sized,
            {
                $(
                    if let Ok(storage) = <$ty>::from_auth_data(json) {
                        return Ok($enum_name::$variant(storage));
                    }
                )+
                Err($crate::error::RlistError::Storage(
                    $crate::error::StorageError::InvalidConfig,
                ))
            }

            fn auth_template(&self) -> String
            where
                Self: Sized,
            {
                match self {
                    $($enum_name::$variant(driver) => <$ty as $crate::Storage>::auth_template(driver),)+
                }
            }
        }
    };
}
