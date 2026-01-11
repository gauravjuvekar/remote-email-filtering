# Register a new app with Google

## Create a new project

Go to [Google cloud console](https://console.cloud.google.com/projectcreate) and
create a new project.


<img src="./google_cloud_console_new_project.png" alt="https://console.cloud.google.com/projectcreate" width="50%"/>


## Configure the [Oauth consent screen](https://console.cloud.google.com/apis/credentials/consent).

- Choose "External" User Type.

    <img src="./google_cloud_console_oauth_consent_screen_1.png" alt="https://console.cloud.google.com/apis/credentials/consent" width="50%"/>

- Fill "App name", and set "User support email" and "Developer contact information" to your Gmail account.

    <img src="./google_cloud_console_oauth_consent_screen_2.png" alt="https://console.cloud.google.com/apis/credentials/consent/edit;newAppInternalUser=false" width="50%"/>

- Manually add the `https://mail.google.com` scope.

    <img src="./google_cloud_console_oauth_consent_screen_3.png" alt="scopes added" width="50%"/>

    <img src="./google_cloud_console_oauth_consent_screen_4.png" alt="scopes added" width="50%"/>

- Add the accounts you want to remotely filter to test users.

    <img src="./google_cloud_console_oauth_consent_screen_5.png" alt="scopes added" width="50%"/>

- Summary

    <img src="./google_cloud_console_oauth_consent_screen_6.png" alt="summary" width="50%"/>


## Get client credentials

- Go to [Credentials](https://console.cloud.google.com/apis/credentials) and
    click on "Create Credentials". Choose "OAuth client ID".

    <img src="./google_cloud_create_credentials_1.png" alt="create credentials" width="50%"/>

- Choose "Desktop app" and name it. (You can experiment with TVs and Limited
    Input devices for device code flow on headless servers.)

- Download the json file and keep it safe.

    <img src="./google_cloud_create_credentials_2.png" alt="create credentials" width="50%"/>

    <img src="./google_cloud_create_credentials_3.png" alt="create credentials" width="50%"/>
