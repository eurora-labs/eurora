use eur_prompt_kit::PromptKitService;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = "Draft a letter to my doctor, Dr. Robert Jones at St. Mary's Hospital, explaining my recent Type II diabetes diagnosis and referencing my latest lab results: cholesterol 230 mg/dL, A1C 8.4%. Include my prescriptions (Lisinopril 10 mg daily) and provide my contact details—phone 512-555-0199, email jane.smith85@example.com—so he can follow up.".to_string();
    // let anonymized_text = PromptKitService::anonymize_text(text).await?;
    // println!("Anonymized text: {}", anonymized_text);
    Ok(())
}
