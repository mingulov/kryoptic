// Copyright 2024 Simo Sorce
// See LICENSE.txt file for terms

use crate::tests::*;

use serial_test::{parallel, serial};

#[test]
#[serial]
fn test_login() {
    let mut testtokn = TestToken::initialized("test_login", None);
    let session = testtokn.get_session(false);

    let mut info = CK_SESSION_INFO {
        slotID: CK_UNAVAILABLE_INFORMATION,
        state: CK_UNAVAILABLE_INFORMATION,
        flags: 0,
        ulDeviceError: 0,
    };
    let ret = fn_get_session_info(session, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RO_PUBLIC_SESSION);

    let mut session2: CK_SESSION_HANDLE = CK_UNAVAILABLE_INFORMATION;
    let ret = fn_open_session(
        testtokn.get_slot(),
        CKF_SERIAL_SESSION | CKF_RW_SESSION,
        std::ptr::null_mut(),
        None,
        &mut session2,
    );
    assert_eq!(ret, CKR_OK);

    let ret = fn_get_session_info(session2, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RW_PUBLIC_SESSION);

    let pin_flags_mask = CKF_SO_PIN_TO_BE_CHANGED
        | CKF_SO_PIN_LOCKED
        | CKF_SO_PIN_FINAL_TRY
        | CKF_SO_PIN_COUNT_LOW
        | CKF_USER_PIN_TO_BE_CHANGED
        | CKF_USER_PIN_LOCKED
        | CKF_USER_PIN_FINAL_TRY
        | CKF_USER_PIN_COUNT_LOW;

    /* check pin flags */
    let mut token_info = CK_TOKEN_INFO::default();
    let ret = fn_get_token_info(testtokn.get_slot(), &mut token_info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(token_info.flags & pin_flags_mask, 0);

    /* fail login first */
    let pin = "87654321";
    let ret = fn_login(
        session,
        CKU_USER,
        pin.as_ptr() as *mut _,
        pin.len() as CK_ULONG,
    );
    assert_ne!(ret, CKR_OK);

    /* check pin flags */
    let mut token_info = CK_TOKEN_INFO::default();
    let ret = fn_get_token_info(testtokn.get_slot(), &mut token_info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(token_info.flags & pin_flags_mask, 0);

    /* fail a few more times to bring the count to low */
    for _ in 1..7 {
        let pin = "87654321";
        let ret = fn_login(
            session,
            CKU_USER,
            pin.as_ptr() as *mut _,
            pin.len() as CK_ULONG,
        );
        assert_ne!(ret, CKR_OK);
    }

    /* NSS DB does not support pin counter */
    if testtokn.dbtype != "nssdb" {
        /* check pin flags */
        let mut token_info = CK_TOKEN_INFO::default();
        let ret = fn_get_token_info(testtokn.get_slot(), &mut token_info);
        assert_eq!(ret, CKR_OK);
        assert_eq!(token_info.flags & pin_flags_mask, CKF_USER_PIN_COUNT_LOW);
    }

    /* login */
    let pin = "12345678";
    let ret = fn_login(
        session,
        CKU_USER,
        pin.as_ptr() as *mut _,
        pin.len() as CK_ULONG,
    );
    assert_eq!(ret, CKR_OK);

    /* check pin flags */
    let mut token_info = CK_TOKEN_INFO::default();
    let ret = fn_get_token_info(testtokn.get_slot(), &mut token_info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(token_info.flags & pin_flags_mask, 0);

    let ret = fn_get_session_info(session, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RO_USER_FUNCTIONS);

    let ret = fn_get_session_info(session2, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RW_USER_FUNCTIONS);

    let ret = fn_login(
        session,
        CKU_USER,
        pin.as_ptr() as *mut _,
        pin.len() as CK_ULONG,
    );
    assert_eq!(ret, CKR_USER_ALREADY_LOGGED_IN);

    let ret = fn_logout(session2);
    assert_eq!(ret, CKR_OK);

    let ret = fn_get_session_info(session, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RO_PUBLIC_SESSION);

    let ret = fn_get_session_info(session2, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RW_PUBLIC_SESSION);

    let ret = fn_logout(session);
    assert_eq!(ret, CKR_USER_NOT_LOGGED_IN);

    let ret = fn_close_session(session2);
    assert_eq!(ret, CKR_OK);

    testtokn.finalize();
}

#[test]
#[parallel]
fn test_login_close() {
    let mut testtokn = TestToken::initialized("test_login_close", None);

    /* Run this twice and make sure the second call does not return ALREADY_LOGGED_IN */
    for _ in 0..2 {
        let session = testtokn.get_session(true);

        let mut info = CK_SESSION_INFO {
            slotID: CK_UNAVAILABLE_INFORMATION,
            state: CK_UNAVAILABLE_INFORMATION,
            flags: 0,
            ulDeviceError: 0,
        };
        let ret = fn_get_session_info(session, &mut info);
        assert_eq!(ret, CKR_OK);
        assert_eq!(info.state, CKS_RW_PUBLIC_SESSION);

        /* login */
        let pin = "12345678";
        let ret = fn_login(
            session,
            CKU_USER,
            pin.as_ptr() as *mut _,
            pin.len() as CK_ULONG,
        );
        assert_eq!(ret, CKR_OK);

        let ret = fn_get_session_info(session, &mut info);
        assert_eq!(ret, CKR_OK);
        assert_eq!(info.state, CKS_RW_USER_FUNCTIONS);

        /* close session should reset the login state */
        testtokn.close_session();
    }
}

#[test]
#[parallel]
fn test_login_close_all() {
    let mut testtokn = TestToken::initialized("test_login_close_all", None);

    let mut session = CK_INVALID_HANDLE;
    let ret = fn_open_session(
        testtokn.get_slot(),
        CKF_SERIAL_SESSION | CKF_RW_SESSION,
        std::ptr::null_mut(),
        None,
        &mut session,
    );
    assert_eq!(ret, CKR_OK);

    let mut info = CK_SESSION_INFO {
        slotID: CK_UNAVAILABLE_INFORMATION,
        state: CK_UNAVAILABLE_INFORMATION,
        flags: 0,
        ulDeviceError: 0,
    };
    let ret = fn_get_session_info(session, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RW_PUBLIC_SESSION);

    /* login */
    let pin = "12345678";
    let ret = fn_login(
        session,
        CKU_USER,
        pin.as_ptr() as *mut _,
        pin.len() as CK_ULONG,
    );
    assert_eq!(ret, CKR_OK);

    let ret = fn_get_session_info(session, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RW_USER_FUNCTIONS);

    /* close session should reset the login state */
    let ret = fn_close_all_sessions(testtokn.get_slot());
    assert_eq!(ret, CKR_OK);

    let session = testtokn.get_session(true);
    let mut info = CK_SESSION_INFO {
        slotID: CK_UNAVAILABLE_INFORMATION,
        state: CK_UNAVAILABLE_INFORMATION,
        flags: 0,
        ulDeviceError: 0,
    };
    let ret = fn_get_session_info(session, &mut info);
    assert_eq!(ret, CKR_OK);
    assert_eq!(info.state, CKS_RW_PUBLIC_SESSION);
}
