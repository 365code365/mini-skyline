// 小程序入口
App({
    globalData: {
        userInfo: null,
        theme: 'light'
    },
    
    onLaunch: function(options) {
        console.log('App onLaunch', options);
        
        // 获取系统信息
        var sysInfo = wx.getSystemInfoSync();
        console.log('System:', sysInfo.platform, sysInfo.windowWidth + 'x' + sysInfo.windowHeight);
        
        // 检查登录状态
        var token = wx.getStorageSync('token');
        if (token) {
            console.log('User logged in');
        }
    },
    
    onShow: function(options) {
        console.log('App onShow');
    },
    
    onHide: function() {
        console.log('App onHide');
    },
    
    onError: function(error) {
        console.error('App error:', error);
    }
});
