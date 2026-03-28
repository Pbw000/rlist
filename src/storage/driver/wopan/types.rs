//! 联通云盘类型定义

use crate::storage::file_meta::Meta;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// 文件类型
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum WopanFileType {
    Folder,
    File,
}

impl<'de> Deserialize<'de> for WopanFileType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => {
                if n.as_i64() == Some(0) {
                    Ok(WopanFileType::Folder)
                } else if n.as_i64() == Some(1) {
                    Ok(WopanFileType::File)
                } else {
                    Err(Error::custom(format!("invalid file type: {}", n)))
                }
            }
            serde_json::Value::String(s) => {
                if s == "0" {
                    Ok(WopanFileType::Folder)
                } else if s == "1" {
                    Ok(WopanFileType::File)
                } else {
                    Err(Error::custom(format!("invalid file type: {}", s)))
                }
            }
            _ => Err(Error::custom("file type must be number or string")),
        }
    }
}

/// 文件元数据 - 只保留必要的字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanFileMeta {
    #[serde(rename = "fid")]
    pub fid: String,
    #[serde(rename = "size", default)]
    pub size: Option<u64>,
    #[serde(rename = "createTime")]
    pub create_time: String,
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "type")]
    pub file_type: WopanFileType,
    #[serde(rename = "name")]
    pub name: String,
}

impl WopanFileMeta {
    pub fn to_meta(&self) -> Meta {
        let modified_at = self.parse_create_time();
        match self.file_type {
            WopanFileType::Folder => Meta::Directory {
                name: self.name.clone(),
                modified_at,
            },
            WopanFileType::File => Meta::File {
                name: self.name.clone(),
                size: self.size.unwrap_or(0),
                modified_at,
            },
        }
    }

    fn parse_create_time(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_str(&self.create_time, "%Y%m%d%H%M%S")
            .map(|d| d.with_timezone(&Utc))
            .ok()
    }
}

/// 文件列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanQueryAllFilesData {
    #[serde(rename = "files")]
    pub files: Vec<WopanFileMeta>,
}

/// API 响应包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanDispatcherResponse {
    #[serde(rename = "STATUS")]
    pub status: String,
    #[serde(rename = "MSG", default)]
    pub msg: Option<String>,
    #[serde(rename = "LOGID", default)]
    pub logid: Option<String>,
    #[serde(rename = "RSP")]
    pub rsp: Option<WopanRsp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanRsp {
    #[serde(rename = "RSP_CODE")]
    pub rsp_code: String,
    #[serde(rename = "RSP_DESC", default)]
    pub rsp_desc: Option<String>,
    #[serde(rename = "DATA", default)]
    pub data: Option<String>,
}

impl WopanDispatcherResponse {
    pub fn into_result(self) -> Result<String, String> {
        // 先检查 STATUS
        if self.status != "200" {
            return Err(self.msg.unwrap_or_else(|| "Request failed".to_string()));
        }

        // 检查 RSP 是否存在
        let rsp = self.rsp.ok_or_else(|| "No RSP in response".to_string())?;

        if rsp.rsp_code == "0000" {
            rsp.data.ok_or_else(|| "No data in response".to_string())
        } else {
            Err(rsp.rsp_desc.unwrap_or_else(|| "API error".to_string()))
        }
    }
}

/// 下载链接响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanDownloadUrlData {
    #[serde(rename = "type", default)]
    pub type_: Option<i32>,
    #[serde(rename = "list")]
    pub list: Vec<WopanDownloadItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanDownloadItem {
    #[serde(rename = "fid")]
    pub fid: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
}

/// 创建文件夹响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanCreateDirectoryData {
    #[serde(rename = "id")]
    pub id: String,
}

/// 上传响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanUpload2CResp {
    #[serde(rename = "data")]
    pub data: WopanUpload2CData,
    #[serde(rename = "uploadUrl", default)]
    pub upload_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanUpload2CData {
    #[serde(rename = "fid")]
    pub fid: String,
}

/// 家庭用户信息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanFamilyUserData {
    #[serde(rename = "defaultHomeId")]
    pub default_home_id: i32,
}

/// 区域信息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanZoneInfoData {
    #[serde(rename = "url")]
    pub url: String,
}

/// 请求头
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanRequestHeader {
    #[serde(rename = "key")]
    pub key: String,
    #[serde(rename = "resTime")]
    pub res_time: u128,
    #[serde(rename = "reqSeq")]
    pub req_seq: u32,
    #[serde(rename = "channel")]
    pub channel: Cow<'static, str>,
    #[serde(rename = "sign")]
    pub sign: String,
    #[serde(rename = "version")]
    pub version: Cow<'static, str>,
}

/// 请求体包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanRequestBody<T> {
    #[serde(rename = "header")]
    pub header: WopanRequestHeader,
    #[serde(rename = "body")]
    pub body: T,
}

/// 加密请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanEncryptedParam {
    #[serde(rename = "param")]
    pub param: String,
    #[serde(rename = "secret")]
    pub secret: bool,
}

// ============== 请求体类型定义 ==============

/// 查询文件列表请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAllFilesBody {
    #[serde(rename = "spaceType")]
    pub space_type: String,
    #[serde(rename = "parentDirectoryId")]
    pub parent_directory_id: String,
    #[serde(rename = "pageNum")]
    pub page_num: i32,
    #[serde(rename = "pageSize")]
    pub page_size: i32,
    #[serde(rename = "sortRule")]
    pub sort_rule: i32,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "familyId", skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
}

/// 获取下载链接请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDownloadUrlV2Body {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "fidList")]
    pub fid_list: Vec<String>,
    #[serde(rename = "clientId")]
    pub client_id: String,
}

/// 创建文件夹请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDirectoryBody {
    #[serde(rename = "spaceType")]
    pub space_type: String,
    #[serde(rename = "familyId", skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
    #[serde(rename = "parentDirectoryId")]
    pub parent_directory_id: String,
    #[serde(rename = "directoryName")]
    pub directory_name: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
}

/// 删除文件请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFileBody {
    #[serde(rename = "spaceType")]
    pub space_type: String,
    #[serde(rename = "vipLevel")]
    pub vip_level: String,
    #[serde(rename = "dirList")]
    pub dir_list: Option<Vec<String>>,
    #[serde(rename = "fileList")]
    pub file_list: Vec<String>,
    #[serde(rename = "clientId")]
    pub client_id: String,
}

/// 重命名请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameFileOrDirectoryBody {
    #[serde(rename = "spaceType")]
    pub space_type: String,
    #[serde(rename = "type")]
    pub type_: i32,
    #[serde(rename = "fileType")]
    pub file_type: String,
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "familyId", skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
}

/// 复制文件请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyFileBody {
    #[serde(rename = "targetDirId")]
    pub target_dir_id: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    #[serde(rename = "targetType")]
    pub target_type: String,
    #[serde(rename = "dirList")]
    pub dir_list: Option<Vec<String>>,
    #[serde(rename = "fileList")]
    pub file_list: Vec<String>,
    #[serde(rename = "secret")]
    pub secret: bool,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "fromFamilyId", skip_serializing_if = "Option::is_none")]
    pub from_family_id: Option<String>,
    #[serde(rename = "familyId", skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
}

/// 移动文件请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveFileBody {
    #[serde(rename = "targetDirId")]
    pub target_dir_id: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    #[serde(rename = "targetType")]
    pub target_type: String,
    #[serde(rename = "dirList")]
    pub dir_list: Option<Vec<String>>,
    #[serde(rename = "fileList")]
    pub file_list: Vec<String>,
    #[serde(rename = "secret")]
    pub secret: bool,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "fromFamilyId", skip_serializing_if = "Option::is_none")]
    pub from_family_id: Option<String>,
    #[serde(rename = "familyId", skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
}

/// 上传创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upload2CBody {
    #[serde(rename = "spaceType")]
    pub space_type: String,
    #[serde(rename = "familyId", skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
    #[serde(rename = "parentDirectoryId")]
    pub parent_directory_id: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "zoneUrl")]
    pub zone_url: String,
    /// 以下为实际上传接口所需的额外字段
    #[serde(rename = "uniqueId", skip_serializing)]
    pub unique_id: Option<String>,
    #[serde(rename = "accessToken", skip_serializing)]
    pub access_token: Option<String>,
    #[serde(rename = "psToken", skip_serializing)]
    pub ps_token: Option<String>,
    #[serde(rename = "totalPart")]
    pub total_part: Option<String>,
    #[serde(rename = "partSize")]
    pub part_size: Option<String>,
    #[serde(rename = "partIndex")]
    pub part_index: Option<String>,
    #[serde(rename = "channel")]
    pub channel: Option<String>,
    #[serde(rename = "directoryId")]
    pub directory_id: Option<String>,
    #[serde(rename = "fileInfo", skip_serializing)]
    pub file_info: Option<String>,
}

/// 家庭用户查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyUserCurrentEncodeBody {
    #[serde(rename = "clientId")]
    pub client_id: String,
}

/// 获取区域信息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetZoneInfoBody {
    #[serde(rename = "appId")]
    pub app_id: String,
}

/// 文件上传信息结构体（用于序列化，使用生命周期避免分配）
#[derive(Debug, Serialize)]
pub struct WopanFileInfo<'a> {
    #[serde(rename = "spaceType")]
    pub space_type: &'a str,
    #[serde(rename = "directoryId")]
    pub directory_id: &'a str,
    #[serde(rename = "batchNo")]
    pub batch_no: &'a str,
    #[serde(rename = "fileName")]
    pub file_name: &'a str,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    #[serde(rename = "fileType")]
    pub file_type: &'a str,
    #[serde(rename = "familyId", skip_serializing_if = "Option::is_none")]
    pub family_id: Option<&'a str>,
}
