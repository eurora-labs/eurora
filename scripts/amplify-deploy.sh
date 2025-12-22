
#!/bin/bash
set -e

APP_ID=$1
BRANCH_NAME=$2

echo "Deploy app $APP_ID branch $BRANCH_NAME"

# Trigger Amplify build job
echo "Triggering Amplify build..."
JOB_ID=$(aws amplify start-job --app-id $APP_ID --branch-name $BRANCH_NAME --job-type RELEASE | jq -r '.jobSummary.jobId')

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
