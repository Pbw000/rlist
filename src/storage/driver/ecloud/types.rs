//! 天翼云盘类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 文件类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EcloudFileType {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "folder")]
    Folder,
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcloudFileMeta {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "type")]
    pub file_type: EcloudFileType,
    #[serde(rename = "size")]
    pub size: Option<u64>,
    #[serde(rename = "lastOpTime")]
    pub last_op_time: Option<String>,
    #[serde(rename = "createDate")]
    pub create_date: Option<String>,
    #[serde(rename = "md5")]
    pub md5: Option<String>,
}

impl EcloudFileMeta {
    /// 转换为统一的 Meta 类型
    pub fn to_meta(&self) -> crate::storage::file_meta::Meta {
        let modified_at = self
            .last_op_time
            .as_ref()
            .or(self.create_date.as_ref())
            .and_then(|time_str| {
                // 尝试解析时间戳（毫秒）
                if let Ok(ts) = time_str.parse::<i64>() {
                    DateTime::from_timestamp_millis(ts)
                } else {
                    // 尝试解析 RFC3339 格式
                    DateTime::parse_from_rfc3339(time_str)
                        .map(|d| d.with_timezone(&Utc))
                        .ok()
                }
            });

        match self.file_type {
            EcloudFileType::File => crate::storage::file_meta::Meta::File {
                name: self.name.clone(),
                size: self.size.unwrap_or(0),
                modified_at,
            },
            EcloudFileType::Folder => crate::storage::file_meta::Meta::Directory {
                name: self.name.clone(),
                modified_at,
            },
        }
    }
}

/// 文件列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcloudFileListResponse {
    #[serde(rename = "fileListAO")]
    pub file_list: FileListAO,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileListAO {
    #[serde(rename = "count")]
    pub count: Option<i64>,
    #[serde(rename = "fileList")]
    pub files: Vec<EcloudFileMeta>,
    #[serde(rename = "folderList")]
    pub folders: Vec<EcloudFileMeta>,
}

impl EcloudFileListResponse {
    pub fn total(&self) -> u64 {
        self.file_list.count.unwrap_or(0) as u64
    }

    pub fn to_file_list(&self) -> crate::storage::model::FileList {
        let mut items = Vec::new();
        for folder in &self.file_list.folders {
            items.push(folder.to_meta());
        }
        for file in &self.file_list.files {
            items.push(file.to_meta());
        }
        crate::storage::model::FileList::new(items, self.total())
    }
}

/// 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcloudErrorResponse {
    #[serde(rename = "res_code")]
    pub res_code: Option<i64>,
    #[serde(rename = "res_message")]
    pub res_message: Option<String>,
    #[serde(rename = "error")]
    pub error: Option<String>,
    #[serde(rename = "code")]
    pub code: Option<String>,
    #[serde(rename = "message")]
    pub message: Option<String>,
    #[serde(rename = "msg")]
    pub msg: Option<String>,
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "errorMsg")]
    pub error_msg: Option<String>,
}

impl EcloudErrorResponse {
    pub fn get_error_message(&self) -> String {
        self.res_message
            .clone()
            .or_else(|| self.error_msg.clone())
            .or_else(|| self.message.clone())
            .or_else(|| self.msg.clone())
            .or_else(|| self.error.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}

/// 通用 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcloudApiResponse<T> {
    #[serde(rename = "res_code")]
    pub res_code: Option<i64>,
    #[serde(rename = "res_message")]
    pub res_message: Option<String>,
    #[serde(rename = "data")]
    pub data: Option<T>,
}

impl<T> EcloudApiResponse<T> {
    pub fn into_result(self) -> Result<T, String> {
        if self.res_code == Some(0) || self.res_code == Some(200) {
            self.data.ok_or_else(|| "No data in response".to_string())
        } else {
            Err(self
                .res_message
                .unwrap_or_else(|| "Unknown error".to_string()))
        }
    }
}

/// 登录响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResp {
    #[serde(rename = "msg")]
    pub msg: Option<String>,
    #[serde(rename = "result")]
    pub result: Option<i64>,
    #[serde(rename = "toUrl")]
    pub to_url: Option<String>,
}

/// 用户 Session 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSessionResp {
    #[serde(rename = "res_code")]
    pub res_code: Option<i64>,
    #[serde(rename = "res_message")]
    pub res_message: Option<String>,
    #[serde(rename = "loginName")]
    pub login_name: Option<String>,
    #[serde(rename = "sessionKey")]
    pub session_key: Option<String>,
    #[serde(rename = "sessionSecret")]
    pub session_secret: Option<String>,
    #[serde(rename = "familySessionKey")]
    pub family_session_key: Option<String>,
    #[serde(rename = "familySessionSecret")]
    pub family_session_secret: Option<String>,
}

/// App Session 响应（包含 Token）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSessionResp {
    #[serde(flatten)]
    pub user_session: UserSessionResp,
    #[serde(rename = "accessToken")]
    pub access_token: Option<String>,
    #[serde(rename = "refreshToken")]
    pub refresh_token: Option<String>,
}

/// 家庭云信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyInfoResp {
    #[serde(rename = "familyId")]
    pub family_id: i64,
    #[serde(rename = "remarkName")]
    pub remark_name: Option<String>,
    #[serde(rename = "count")]
    pub count: Option<i64>,
}

/// 家庭云列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyInfoListResp {
    #[serde(rename = "familyInfoResp")]
    pub family_info_list: Vec<FamilyInfoResp>,
}

/// 创建文件夹响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFolderResp {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
}

/// 下载 URL 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadUrlResp {
    #[serde(rename = "fileDownloadUrl")]
    pub download_url: String,
}

/// 批量任务响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBatchTaskResp {
    #[serde(rename = "taskId")]
    pub task_id: String,
}

/// 批量任务状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskStateResp {
    #[serde(rename = "taskStatus")]
    pub task_status: i64,
    #[serde(rename = "process")]
    pub process: i64,
    #[serde(rename = "successedCount")]
    pub success_count: i64,
    #[serde(rename = "failedCount")]
    pub failed_count: i64,
    #[serde(rename = "subTaskCount")]
    pub sub_task_count: i64,
}

/// 上传初始化响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitUploadResp {
    #[serde(rename = "uploadFileId")]
    pub upload_file_id: Option<String>,
    #[serde(rename = "fileUploadUrl")]
    pub file_upload_url: Option<String>,
    #[serde(rename = "fileCommitUrl")]
    pub file_commit_url: Option<String>,
    #[serde(rename = "fileDataExists")]
    pub file_data_exists: Option<i64>,
    #[serde(rename = "uploadType")]
    pub upload_type: Option<i64>,
}

/// 上传提交响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitUploadResp {
    #[serde(rename = "file")]
    pub file: Option<UploadedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedFile {
    #[serde(rename = "userFileId")]
    pub user_file_id: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    #[serde(rename = "fileMd5")]
    pub file_md5: String,
}
