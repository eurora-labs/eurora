
#!/bin/bash
set -e

APP_ID=$1
BRANCH_NAME=$2
ZIP_FILE=$3

echo "Deploy app $APP_ID branch $BRANCH_NAME"

if [ -n "$ZIP_FILE" ] && [ -f "$ZIP_FILE" ]; then
    echo "Deploying pre-built artifacts from $ZIP_FILE"
    
    # Create a deployment and get the upload URL
    DEPLOYMENT_RESPONSE=$(aws amplify create-deployment --app-id $APP_ID --branch-name $BRANCH_NAME)
    JOB_ID=$(echo $DEPLOYMENT_RESPONSE | jq -r '.jobId')
    UPLOAD_URL=$(echo $DEPLOYMENT_RESPONSE | jq -r '.zipUploadUrl')
    
    echo "Job ID is $JOB_ID"
    echo "Uploading deployment artifact..."
    
    # Upload the zip file to the presigned URL
    curl --request PUT --upload-file "$ZIP_FILE" "$UPLOAD_URL" --fail --silent --show-error
    
    echo "Upload complete. Starting deployment..."
    
    # Start the deployment
    aws amplify start-deployment --app-id $APP_ID --branch-name $BRANCH_NAME --job-id $JOB_ID
else
    echo "No zip file provided, triggering Amplify build..."
    JOB_ID=$(aws amplify start-job --app-id $APP_ID --branch-name $BRANCH_NAME --job-type RELEASE | jq -r '.jobSummary.jobId')
fi

echo "Release started"
echo "Job ID is $JOB_ID"

# Wait for the deployment to complete
while [[ "$(aws amplify get-job --app-id $APP_ID --branch-name $BRANCH_NAME --job-id $JOB_ID | jq -r '.job.summary.status')" =~ ^(PENDING|RUNNING)$ ]]; do
    echo "Deployment in progress..."
    sleep 5
done

JOB_STATUS="$(aws amplify get-job --app-id $APP_ID --branch-name $BRANCH_NAME --job-id $JOB_ID | jq -r '.job.summary.status')"
echo "Job finished"
echo "Job status is $JOB_STATUS"

if [ "$JOB_STATUS" != "SUCCEED" ]; then
    echo "Deployment failed with status: $JOB_STATUS"
    exit 1
fi

echo "Deployment successful!"
