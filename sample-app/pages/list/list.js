// 列表页
Page({
    data: {
        items: [
            { id: 1, title: '商品一', desc: '这是商品一的描述信息' },
            { id: 2, title: '商品二', desc: '这是商品二的描述信息' },
            { id: 3, title: '商品三', desc: '这是商品三的描述信息' },
            { id: 4, title: '商品四', desc: '这是商品四的描述信息' },
            { id: 5, title: '商品五', desc: '这是商品五的描述信息' }
        ]
    },
    
    onLoad: function(options) {
        __native_print('List page onLoad');
        if (options && options.from) {
            __native_print('From: ' + options.from);
        }
    },
    
    onShow: function() {
        __native_print('List page onShow');
    },
    
    // 点击列表项，跳转到详情页
    onItemTap: function(e) {
        var id = e.currentTarget.dataset.id;
        __native_print('Item tapped: ' + id);
        wx.navigateTo({
            url: '/pages/detail/detail?id=' + id
        });
    },
    
    // 返回首页
    onBackHome: function() {
        __native_print('Back to home');
        wx.switchTab({
            url: '/pages/index/index'
        });
    }
});
