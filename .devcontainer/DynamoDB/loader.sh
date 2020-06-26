#!/bin/sh
for filename in $DYNAMO_FOLDER/tables/*.json; do
    printf "\nCreating Table (%s) in Dynamodb" "$filename"
    AWS_ACCESS_KEY_ID=MOCK \
    AWS_SECRET_ACCESS_KEY=MOCK \
    aws dynamodb create-table --cli-input-json "file://$filename" \
    --endpoint-url $ENDPOINT_URL --region $REGION
done

if [ -f "$DYNAMO_FOLDER/batchinput.json" ]; then
    printf "\nBatch Writing (%s)" "$DYNAMO_FOLDER/batchinput.json"
    AWS_ACCESS_KEY_ID=MOCK \
    AWS_SECRET_ACCESS_KEY=MOCK \
    aws dynamodb batch-write-item --cli-input-json "file://$DYNAMO_FOLDER/batchinput.json" \
    --endpoint-url $ENDPOINT_URL --region $REGION
fi

exit 0;