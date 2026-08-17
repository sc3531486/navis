use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum ContentPart { Text(TextContent), Image(ImageContent), File(FileContent) }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct TextContent { pub text: String }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ImageContent { pub media_type: String, pub data: String }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct FileContent { pub file_name: String, pub content: String }
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum ImageMediaType { Png, Jpeg, Gif, WebP }
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum ImageSourceType { Base64, Url }
